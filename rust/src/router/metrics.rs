use pyo3::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

struct RoutingRecord {
    model: String,
    tier: String,
    confidence: f32,
    cost_estimate: f64,
    timestamp_ms: u64,
}

struct RouterMetrics {
    total_calls: u64,
    escalation_count: u64,
    tier_counts: HashMap<String, u64>,
    model_counts: HashMap<String, u64>,
    total_estimated_cost: f64,
    records: Vec<RoutingRecord>,
}

impl Default for RouterMetrics {
    fn default() -> Self {
        Self {
            total_calls: 0,
            escalation_count: 0,
            tier_counts: HashMap::new(),
            model_counts: HashMap::new(),
            total_estimated_cost: 0.0,
            records: Vec::new(),
        }
    }
}

fn get_metrics() -> &'static Mutex<RouterMetrics> {
    static METRICS: OnceLock<Mutex<RouterMetrics>> = OnceLock::new();
    METRICS.get_or_init(|| Mutex::new(RouterMetrics::default()))
}

/// Record a routing decision into the global metrics store.
pub fn record_decision(model: &str, tier: &str, confidence: f32, cost_estimate: f64) {
    let Ok(mut m) = get_metrics().lock() else {
        return;
    };
    m.total_calls += 1;
    *m.tier_counts.entry(tier.to_string()).or_insert(0) += 1;
    *m.model_counts.entry(model.to_string()).or_insert(0) += 1;
    m.total_estimated_cost += cost_estimate;
    m.records.push(RoutingRecord {
        model: model.to_string(),
        tier: tier.to_string(),
        confidence,
        cost_estimate,
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    });
}

/// Record a tier escalation event.
#[pyfunction]
pub fn record_escalation() -> PyResult<()> {
    let Ok(mut m) = get_metrics().lock() else {
        return Ok(());
    };
    m.escalation_count += 1;
    Ok(())
}

/// Return full metrics summary as JSON.
#[pyfunction]
pub fn get_router_metrics() -> PyResult<String> {
    let m = get_metrics()
        .lock()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("lock poisoned: {e}")))?;

    let last_decision = m.records.last().map(|r| {
        json!({
            "model": r.model,
            "tier": r.tier,
            "confidence": r.confidence,
            "cost_estimate": r.cost_estimate,
        })
    });

    let result = json!({
        "total_calls": m.total_calls,
        "escalation_count": m.escalation_count,
        "tier_counts": m.tier_counts,
        "model_counts": m.model_counts,
        "total_estimated_cost": m.total_estimated_cost,
        "last_decision": last_decision,
    });
    Ok(result.to_string())
}

/// Reset all metrics (useful for tests or session boundaries).
#[pyfunction]
pub fn reset_router_metrics() -> PyResult<()> {
    let mut m = get_metrics()
        .lock()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("lock poisoned: {e}")))?;
    *m = RouterMetrics::default();
    Ok(())
}

/// Lightweight: return just the total call count.
#[pyfunction]
pub fn get_router_metrics_count() -> PyResult<u64> {
    let m = get_metrics()
        .lock()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("lock poisoned: {e}")))?;
    Ok(m.total_calls)
}
