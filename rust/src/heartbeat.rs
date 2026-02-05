//! Heartbeat service - periodic agent wake-up to check for tasks.

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default interval: 30 minutes
const DEFAULT_HEARTBEAT_INTERVAL_S: u64 = 30 * 60;

/// The prompt sent to agent during heartbeat
const HEARTBEAT_PROMPT: &str = r#"Read HEARTBEAT.md in your workspace (if it exists).
Follow any instructions or tasks listed there.
If nothing needs attention, reply with just: HEARTBEAT_OK"#;

/// Token that indicates "nothing to do"
const HEARTBEAT_OK_TOKEN: &str = "HEARTBEAT_OK";

/// Check if HEARTBEAT.md has no actionable content.
fn is_heartbeat_empty(content: Option<&str>) -> bool {
    let content = match content {
        Some(c) if !c.is_empty() => c,
        _ => return true,
    };

    // Lines to skip: empty, headers, HTML comments, checkboxes
    let skip_patterns = ["- [ ]", "* [ ]", "- [x]", "* [x]"];

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("<!--")
            || skip_patterns.contains(&line)
        {
            continue;
        }
        return false; // Found actionable content
    }

    true
}

/// Periodic heartbeat service that wakes the agent to check for tasks.
///
/// The agent reads HEARTBEAT.md from the workspace and executes any
/// tasks listed there. If nothing needs attention, it replies HEARTBEAT_OK.
#[pyclass]
pub struct HeartbeatService {
    workspace: PathBuf,
    callback: Arc<Mutex<Option<PyObject>>>,
    interval_s: u64,
    enabled: bool,
    running: Arc<AtomicBool>,
}

#[pymethods]
impl HeartbeatService {
    #[new]
    #[pyo3(signature = (workspace, on_heartbeat=None, interval_s=None, enabled=true))]
    fn new(
        workspace: PathBuf,
        on_heartbeat: Option<PyObject>,
        interval_s: Option<u64>,
        enabled: bool,
    ) -> Self {
        Self {
            workspace,
            callback: Arc::new(Mutex::new(on_heartbeat)),
            interval_s: interval_s.unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_S),
            enabled,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the heartbeat file path.
    #[getter]
    fn heartbeat_file(&self) -> String {
        self.workspace
            .join("HEARTBEAT.md")
            .to_string_lossy()
            .to_string()
    }

    /// Set the callback function.
    #[allow(unused_variables)]
    fn set_callback(&self, py: Python<'_>, callback: Option<PyObject>) -> PyResult<()> {
        let cb = self.callback.clone();
        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
            let mut guard = cb.lock().await;
            *guard = callback;
        });
        Ok(())
    }

    /// Start the heartbeat service.
    fn start<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if !self.enabled {
            // Return immediately if disabled
            return future_into_py(py, async { Ok(()) });
        }

        self.running.store(true, Ordering::Relaxed);

        let workspace = self.workspace.clone();
        let callback = self.callback.clone();
        let interval_s = self.interval_s;
        let running = self.running.clone();

        future_into_py(py, async move {
            eprintln!("[heartbeat] Started (every {}s)", interval_s);

            while running.load(Ordering::Relaxed) {
                // Sleep first (heartbeat fires after interval)
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_s)).await;

                if !running.load(Ordering::Relaxed) {
                    break;
                }

                // Execute tick
                if let Err(e) = tick_inner(&workspace, &callback).await {
                    eprintln!("[heartbeat] Error: {}", e);
                }
            }

            Ok(())
        })
    }

    /// Stop the heartbeat service.
    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Check if the service is running.
    #[getter]
    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Manually trigger a heartbeat.
    fn trigger_now<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let callback = self.callback.clone();

        future_into_py(py, async move {
            let guard = callback.lock().await;
            if let Some(cb) = guard.as_ref() {
                // Clone the callback inside GIL
                let cb_clone: PyObject = Python::with_gil(|py| cb.clone_ref(py));
                drop(guard);

                let response: PyResult<String> = Python::with_gil(|py| {
                    let coro = cb_clone.call1(py, (HEARTBEAT_PROMPT,))?;
                    let bound = coro.into_bound(py);
                    let future = pyo3_async_runtimes::tokio::into_future(bound)?;

                    pyo3_async_runtimes::tokio::get_runtime().block_on(async {
                        let result = future.await?;
                        Python::with_gil(|py| result.extract::<String>(py))
                    })
                });

                return Ok(Some(response?));
            }
            Ok(None)
        })
    }

    /// Get interval in seconds.
    #[getter]
    fn interval_s(&self) -> u64 {
        self.interval_s
    }

    /// Check if enabled.
    #[getter]
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn __repr__(&self) -> String {
        format!(
            "HeartbeatService(workspace={:?}, interval={}s, enabled={}, running={})",
            self.workspace,
            self.interval_s,
            self.enabled,
            self.is_running()
        )
    }
}

/// Read HEARTBEAT.md content from workspace.
fn read_heartbeat_file(workspace: &Path) -> Option<String> {
    let path = workspace.join("HEARTBEAT.md");
    std::fs::read_to_string(path).ok()
}

/// Execute a single heartbeat tick.
async fn tick_inner(
    workspace: &Path,
    callback: &Arc<Mutex<Option<PyObject>>>,
) -> Result<(), String> {
    let content = read_heartbeat_file(workspace);

    // Skip if HEARTBEAT.md is empty or doesn't exist
    if is_heartbeat_empty(content.as_deref()) {
        return Ok(());
    }

    eprintln!("[heartbeat] Checking for tasks...");

    let guard = callback.lock().await;
    if let Some(cb) = guard.as_ref() {
        let cb_clone = Python::with_gil(|py| cb.clone_ref(py));
        drop(guard);

        // Call the Python async callback
        let response = Python::with_gil(|py| -> PyResult<String> {
            let coro = cb_clone.call1(py, (HEARTBEAT_PROMPT,))?;
            let bound = coro.into_bound(py);
            let future = pyo3_async_runtimes::tokio::into_future(bound)?;

            pyo3_async_runtimes::tokio::get_runtime().block_on(async {
                let result = future.await?;
                Python::with_gil(|py| result.extract::<String>(py))
            })
        })
        .map_err(|e| format!("Callback error: {}", e))?;

        // Check if agent said "nothing to do"
        let normalized = response.to_uppercase().replace('_', "");
        let token_normalized = HEARTBEAT_OK_TOKEN.replace('_', "");
        if normalized.contains(&token_normalized) {
            eprintln!("[heartbeat] OK (no action needed)");
        } else {
            eprintln!("[heartbeat] Completed task");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heartbeat_empty() {
        assert!(is_heartbeat_empty(None));
        assert!(is_heartbeat_empty(Some("")));
        assert!(is_heartbeat_empty(Some("# Header\n\n")));
        assert!(is_heartbeat_empty(Some("<!-- comment -->\n")));
        assert!(is_heartbeat_empty(Some("- [ ]")));
        assert!(!is_heartbeat_empty(Some("Do something")));
        assert!(!is_heartbeat_empty(Some("# Header\nDo something")));
    }
}
