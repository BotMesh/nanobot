//! Cron service for scheduling agent tasks.

use chrono::Utc;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Schedule definition for a cron job.
#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CronSchedule {
    #[pyo3(get, set)]
    pub kind: String, // "at", "every", "cron"
    #[pyo3(get, set)]
    pub at_ms: Option<i64>,
    #[pyo3(get, set)]
    pub every_ms: Option<i64>,
    #[pyo3(get, set)]
    pub expr: Option<String>,
    #[pyo3(get, set)]
    pub tz: Option<String>,
}

#[pymethods]
impl CronSchedule {
    #[new]
    #[pyo3(signature = (kind, at_ms=None, every_ms=None, expr=None, tz=None))]
    fn new(
        kind: String,
        at_ms: Option<i64>,
        every_ms: Option<i64>,
        expr: Option<String>,
        tz: Option<String>,
    ) -> Self {
        Self {
            kind,
            at_ms,
            every_ms,
            expr,
            tz,
        }
    }

    fn __repr__(&self) -> String {
        format!("CronSchedule(kind={:?})", self.kind)
    }
}

/// What to do when the job runs.
#[pyclass]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CronPayload {
    #[pyo3(get, set)]
    pub kind: String, // "system_event", "agent_turn"
    #[pyo3(get, set)]
    pub message: String,
    #[pyo3(get, set)]
    pub deliver: bool,
    #[pyo3(get, set)]
    pub channel: Option<String>,
    #[pyo3(get, set)]
    pub to: Option<String>,
}

#[pymethods]
impl CronPayload {
    #[new]
    #[pyo3(signature = (kind="agent_turn", message="", deliver=false, channel=None, to=None))]
    fn new(
        kind: &str,
        message: &str,
        deliver: bool,
        channel: Option<String>,
        to: Option<String>,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.to_string(),
            deliver,
            channel,
            to,
        }
    }
}

/// Runtime state of a job.
#[pyclass]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CronJobState {
    #[pyo3(get, set)]
    pub next_run_at_ms: Option<i64>,
    #[pyo3(get, set)]
    pub last_run_at_ms: Option<i64>,
    #[pyo3(get, set)]
    pub last_status: Option<String>, // "ok", "error", "skipped"
    #[pyo3(get, set)]
    pub last_error: Option<String>,
}

#[pymethods]
impl CronJobState {
    #[new]
    #[pyo3(signature = (next_run_at_ms=None, last_run_at_ms=None, last_status=None, last_error=None))]
    fn new(
        next_run_at_ms: Option<i64>,
        last_run_at_ms: Option<i64>,
        last_status: Option<String>,
        last_error: Option<String>,
    ) -> Self {
        Self {
            next_run_at_ms,
            last_run_at_ms,
            last_status,
            last_error,
        }
    }
}

/// A scheduled job.
#[pyclass]
#[derive(Clone, Debug)]
pub struct CronJob {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub enabled: bool,
    #[pyo3(get)]
    pub schedule: CronSchedule,
    #[pyo3(get)]
    pub payload: CronPayload,
    #[pyo3(get)]
    pub state: CronJobState,
    #[pyo3(get, set)]
    pub created_at_ms: i64,
    #[pyo3(get, set)]
    pub updated_at_ms: i64,
    #[pyo3(get, set)]
    pub delete_after_run: bool,
}

#[pymethods]
impl CronJob {
    #[new]
    #[pyo3(signature = (id, name, enabled=true, schedule=None, payload=None, state=None, created_at_ms=0, updated_at_ms=0, delete_after_run=false))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: String,
        name: String,
        enabled: bool,
        schedule: Option<CronSchedule>,
        payload: Option<CronPayload>,
        state: Option<CronJobState>,
        created_at_ms: i64,
        updated_at_ms: i64,
        delete_after_run: bool,
    ) -> Self {
        Self {
            id,
            name,
            enabled,
            schedule: schedule
                .unwrap_or_else(|| CronSchedule::new("every".to_string(), None, None, None, None)),
            payload: payload
                .unwrap_or_else(|| CronPayload::new("agent_turn", "", false, None, None)),
            state: state.unwrap_or_default(),
            created_at_ms,
            updated_at_ms,
            delete_after_run,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "CronJob(id={:?}, name={:?}, enabled={})",
            self.id, self.name, self.enabled
        )
    }
}

/// JSON structure for serialization
#[derive(Serialize, Deserialize)]
struct CronStoreJson {
    version: i32,
    jobs: Vec<CronJobJson>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CronJobJson {
    id: String,
    name: String,
    enabled: bool,
    schedule: CronScheduleJson,
    payload: CronPayloadJson,
    state: CronJobStateJson,
    created_at_ms: i64,
    updated_at_ms: i64,
    delete_after_run: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CronScheduleJson {
    kind: String,
    at_ms: Option<i64>,
    every_ms: Option<i64>,
    expr: Option<String>,
    tz: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CronPayloadJson {
    kind: String,
    message: String,
    deliver: bool,
    channel: Option<String>,
    to: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CronJobStateJson {
    next_run_at_ms: Option<i64>,
    last_run_at_ms: Option<i64>,
    last_status: Option<String>,
    last_error: Option<String>,
}

/// Compute next run time in ms.
fn compute_next_run(schedule: &CronSchedule, now_ms: i64) -> Option<i64> {
    match schedule.kind.as_str() {
        "at" => {
            if let Some(at) = schedule.at_ms {
                if at > now_ms {
                    return Some(at);
                }
            }
            None
        }
        "every" => {
            if let Some(every) = schedule.every_ms {
                if every > 0 {
                    return Some(now_ms + every);
                }
            }
            None
        }
        "cron" => {
            if let Some(expr) = &schedule.expr {
                if let Ok(cron_schedule) = cron::Schedule::from_str(expr) {
                    if let Some(next) = cron_schedule.upcoming(Utc).next() {
                        return Some(next.timestamp_millis());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

use std::str::FromStr;

/// Service for managing and executing scheduled jobs.
#[pyclass]
#[allow(dead_code)]
pub struct CronService {
    store_path: PathBuf,
    callback: Arc<Mutex<Option<PyObject>>>,
    jobs: Arc<Mutex<Vec<CronJob>>>,
    running: Arc<AtomicBool>,
}

#[pymethods]
impl CronService {
    #[new]
    #[pyo3(signature = (store_path, on_job=None))]
    fn new(store_path: PathBuf, on_job: Option<PyObject>) -> Self {
        Self {
            store_path,
            callback: Arc::new(Mutex::new(on_job)),
            jobs: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
        }
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

    /// Start the cron service.
    fn start<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.running.store(true, Ordering::Relaxed);

        let store_path = self.store_path.clone();
        let jobs = self.jobs.clone();
        let callback = self.callback.clone();
        let running = self.running.clone();

        future_into_py(py, async move {
            // Load jobs from disk
            {
                let loaded = load_store(&store_path);
                let mut guard = jobs.lock().await;
                *guard = loaded;
            }

            // Recompute next runs
            {
                let now = now_ms();
                let mut guard = jobs.lock().await;
                for job in guard.iter_mut() {
                    if job.enabled {
                        job.state.next_run_at_ms = compute_next_run(&job.schedule, now);
                    }
                }
            }

            // Save store
            save_store(&store_path, &jobs).await;

            let job_count = jobs.lock().await.len();
            eprintln!("[cron] Service started with {} jobs", job_count);

            // Main loop
            while running.load(Ordering::Relaxed) {
                let next_wake = {
                    let guard = jobs.lock().await;
                    guard
                        .iter()
                        .filter(|j| j.enabled && j.state.next_run_at_ms.is_some())
                        .filter_map(|j| j.state.next_run_at_ms)
                        .min()
                };

                let delay_ms = match next_wake {
                    Some(wake) => (wake - now_ms()).max(0) as u64,
                    None => 60000, // Default 1 minute check interval
                };

                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

                if !running.load(Ordering::Relaxed) {
                    break;
                }

                // Execute due jobs
                let now = now_ms();
                let due_job_ids: Vec<String> = {
                    let guard = jobs.lock().await;
                    guard
                        .iter()
                        .filter(|j| {
                            j.enabled
                                && j.state.next_run_at_ms.is_some()
                                && now >= j.state.next_run_at_ms.unwrap()
                        })
                        .map(|j| j.id.clone())
                        .collect()
                };

                for job_id in due_job_ids {
                    execute_job(&jobs, &callback, &job_id).await;
                }

                save_store(&store_path, &jobs).await;
            }

            Ok(())
        })
    }

    /// Stop the cron service.
    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// List all jobs.
    #[pyo3(signature = (include_disabled=false))]
    fn list_jobs<'py>(
        &self,
        py: Python<'py>,
        include_disabled: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let jobs = self.jobs.clone();

        future_into_py(py, async move {
            let guard = jobs.lock().await;
            let mut result: Vec<CronJob> = if include_disabled {
                guard.clone()
            } else {
                guard.iter().filter(|j| j.enabled).cloned().collect()
            };

            // Sort by next_run_at_ms
            result.sort_by_key(|j| j.state.next_run_at_ms.unwrap_or(i64::MAX));
            Ok(result)
        })
    }

    /// Add a new job.
    #[pyo3(signature = (name, schedule, message, deliver=false, channel=None, to=None, delete_after_run=false))]
    #[allow(clippy::too_many_arguments)]
    fn add_job<'py>(
        &self,
        py: Python<'py>,
        name: String,
        schedule: CronSchedule,
        message: String,
        deliver: bool,
        channel: Option<String>,
        to: Option<String>,
        delete_after_run: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let jobs = self.jobs.clone();
        let store_path = self.store_path.clone();

        future_into_py(py, async move {
            let now = now_ms();
            let job = CronJob {
                id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                name: name.clone(),
                enabled: true,
                schedule: schedule.clone(),
                payload: CronPayload {
                    kind: "agent_turn".to_string(),
                    message,
                    deliver,
                    channel,
                    to,
                },
                state: CronJobState {
                    next_run_at_ms: compute_next_run(&schedule, now),
                    ..Default::default()
                },
                created_at_ms: now,
                updated_at_ms: now,
                delete_after_run,
            };

            let job_clone = job.clone();
            {
                let mut guard = jobs.lock().await;
                guard.push(job);
            }

            save_store(&store_path, &jobs).await;
            eprintln!("[cron] Added job '{}' ({})", name, job_clone.id);

            Ok(job_clone)
        })
    }

    /// Remove a job by ID.
    fn remove_job<'py>(&self, py: Python<'py>, job_id: String) -> PyResult<Bound<'py, PyAny>> {
        let jobs = self.jobs.clone();
        let store_path = self.store_path.clone();

        future_into_py(py, async move {
            let removed = {
                let mut guard = jobs.lock().await;
                let before = guard.len();
                guard.retain(|j| j.id != job_id);
                guard.len() < before
            };

            if removed {
                save_store(&store_path, &jobs).await;
                eprintln!("[cron] Removed job {}", job_id);
            }

            Ok(removed)
        })
    }

    /// Enable or disable a job.
    #[pyo3(signature = (job_id, enabled=true))]
    fn enable_job<'py>(
        &self,
        py: Python<'py>,
        job_id: String,
        enabled: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let jobs = self.jobs.clone();
        let store_path = self.store_path.clone();

        future_into_py(py, async move {
            let mut guard = jobs.lock().await;
            for job in guard.iter_mut() {
                if job.id == job_id {
                    job.enabled = enabled;
                    job.updated_at_ms = now_ms();
                    if enabled {
                        job.state.next_run_at_ms = compute_next_run(&job.schedule, now_ms());
                    } else {
                        job.state.next_run_at_ms = None;
                    }
                    let job_clone = job.clone();
                    drop(guard);
                    save_store(&store_path, &jobs).await;
                    return Ok(Some(job_clone));
                }
            }
            Ok(None)
        })
    }

    /// Manually run a job.
    #[pyo3(signature = (job_id, force=false))]
    fn run_job<'py>(
        &self,
        py: Python<'py>,
        job_id: String,
        force: bool,
    ) -> PyResult<Bound<'py, PyAny>> {
        let jobs = self.jobs.clone();
        let callback = self.callback.clone();
        let store_path = self.store_path.clone();

        future_into_py(py, async move {
            let job_exists = {
                let guard = jobs.lock().await;
                guard.iter().any(|j| j.id == job_id && (force || j.enabled))
            };

            if !job_exists {
                return Ok(false);
            }

            execute_job(&jobs, &callback, &job_id).await;
            save_store(&store_path, &jobs).await;
            Ok(true)
        })
    }

    /// Get service status.
    fn status<'py>(&self, py: Python<'py>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("enabled", self.running.load(Ordering::Relaxed))?;

        let jobs = self.jobs.clone();
        let (job_count, next_wake) = pyo3_async_runtimes::tokio::get_runtime().block_on(async {
            let guard = jobs.lock().await;
            let count = guard.len();
            let wake = guard
                .iter()
                .filter(|j| j.enabled && j.state.next_run_at_ms.is_some())
                .filter_map(|j| j.state.next_run_at_ms)
                .min();
            (count, wake)
        });

        dict.set_item("jobs", job_count)?;
        dict.set_item("next_wake_at_ms", next_wake)?;

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        let running = self.running.load(Ordering::Relaxed);
        format!(
            "CronService(store_path={:?}, running={})",
            self.store_path, running
        )
    }
}

/// Load jobs from disk.
fn load_store(path: &Path) -> Vec<CronJob> {
    if !path.exists() {
        return Vec::new();
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let store: CronStoreJson = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    store
        .jobs
        .into_iter()
        .map(|j| CronJob {
            id: j.id,
            name: j.name,
            enabled: j.enabled,
            schedule: CronSchedule {
                kind: j.schedule.kind,
                at_ms: j.schedule.at_ms,
                every_ms: j.schedule.every_ms,
                expr: j.schedule.expr,
                tz: j.schedule.tz,
            },
            payload: CronPayload {
                kind: j.payload.kind,
                message: j.payload.message,
                deliver: j.payload.deliver,
                channel: j.payload.channel,
                to: j.payload.to,
            },
            state: CronJobState {
                next_run_at_ms: j.state.next_run_at_ms,
                last_run_at_ms: j.state.last_run_at_ms,
                last_status: j.state.last_status,
                last_error: j.state.last_error,
            },
            created_at_ms: j.created_at_ms,
            updated_at_ms: j.updated_at_ms,
            delete_after_run: j.delete_after_run,
        })
        .collect()
}

/// Save jobs to disk.
async fn save_store(path: &Path, jobs: &Arc<Mutex<Vec<CronJob>>>) {
    let guard = jobs.lock().await;

    let store = CronStoreJson {
        version: 1,
        jobs: guard
            .iter()
            .map(|j| CronJobJson {
                id: j.id.clone(),
                name: j.name.clone(),
                enabled: j.enabled,
                schedule: CronScheduleJson {
                    kind: j.schedule.kind.clone(),
                    at_ms: j.schedule.at_ms,
                    every_ms: j.schedule.every_ms,
                    expr: j.schedule.expr.clone(),
                    tz: j.schedule.tz.clone(),
                },
                payload: CronPayloadJson {
                    kind: j.payload.kind.clone(),
                    message: j.payload.message.clone(),
                    deliver: j.payload.deliver,
                    channel: j.payload.channel.clone(),
                    to: j.payload.to.clone(),
                },
                state: CronJobStateJson {
                    next_run_at_ms: j.state.next_run_at_ms,
                    last_run_at_ms: j.state.last_run_at_ms,
                    last_status: j.state.last_status.clone(),
                    last_error: j.state.last_error.clone(),
                },
                created_at_ms: j.created_at_ms,
                updated_at_ms: j.updated_at_ms,
                delete_after_run: j.delete_after_run,
            })
            .collect(),
    };

    drop(guard);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let content = match serde_json::to_string_pretty(&store) {
        Ok(c) => c,
        Err(_) => return,
    };

    let _ = std::fs::write(path, content);
}

/// Execute a single job.
async fn execute_job(
    jobs: &Arc<Mutex<Vec<CronJob>>>,
    callback: &Arc<Mutex<Option<PyObject>>>,
    job_id: &str,
) {
    let start_ms = now_ms();

    // Get job info
    let job_info = {
        let guard = jobs.lock().await;
        guard.iter().find(|j| j.id == job_id).cloned()
    };

    let job = match job_info {
        Some(j) => j,
        None => return,
    };

    eprintln!("[cron] Executing job '{}' ({})", job.name, job.id);

    // Call callback if set
    let result: Result<(), String> = {
        let guard = callback.lock().await;
        if let Some(cb) = guard.as_ref() {
            let cb_clone: PyObject = Python::with_gil(|py| cb.clone_ref(py));
            drop(guard);

            Python::with_gil(|py| -> PyResult<()> {
                // Pass the job to the callback
                let job_clone = job.clone();
                let coro = cb_clone.call1(py, (job_clone,))?;
                let bound = coro.into_bound(py);
                let future = pyo3_async_runtimes::tokio::into_future(bound)?;

                pyo3_async_runtimes::tokio::get_runtime().block_on(async {
                    let _ = future.await?;
                    Ok(())
                })
            })
            .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    };

    // Update job state
    {
        let mut guard = jobs.lock().await;
        if let Some(job) = guard.iter_mut().find(|j| j.id == job_id) {
            job.state.last_run_at_ms = Some(start_ms);
            job.updated_at_ms = now_ms();

            match &result {
                Ok(()) => {
                    job.state.last_status = Some("ok".to_string());
                    job.state.last_error = None;
                    eprintln!("[cron] Job '{}' completed", job.name);
                }
                Err(e) => {
                    job.state.last_status = Some("error".to_string());
                    job.state.last_error = Some(e.clone());
                    eprintln!("[cron] Job '{}' failed: {}", job.name, e);
                }
            }

            // Handle one-shot jobs
            if job.schedule.kind == "at" {
                if job.delete_after_run {
                    let job_id = job.id.clone();
                    drop(guard);
                    let mut guard = jobs.lock().await;
                    guard.retain(|j| j.id != job_id);
                } else {
                    job.enabled = false;
                    job.state.next_run_at_ms = None;
                }
            } else {
                // Compute next run
                job.state.next_run_at_ms = compute_next_run(&job.schedule, now_ms());
            }
        }
    }
}
