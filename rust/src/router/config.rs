use std::collections::HashMap;

pub fn default_weights() -> HashMap<&'static str, f32> {
    let mut m = HashMap::new();
    m.insert("reasoning", 0.22);
    m.insert("code", 0.18);
    m.insert("multistep", 0.15);
    m.insert("technical", 0.12);
    m.insert("token_count", 0.10);
    m.insert("creative", 0.06);
    m.insert("question", 0.05);
    m.insert("imperative", 0.04);
    m.insert("format", 0.04);
    m.insert("negation", 0.04);
    m
}

pub fn tier_model_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("SIMPLE", "openai/gpt-3.5-turbo");
    m.insert("MEDIUM", "openai/gpt-4o-mini");
    m.insert("COMPLEX", "anthropic/claude-opus-4-5");
    m.insert("REASONING", "openai/o3");
    m
}

/// Ordered tier list from lowest to highest complexity.
pub const TIER_ORDER: [&str; 4] = ["SIMPLE", "MEDIUM", "COMPLEX", "REASONING"];

/// Returns the next higher tier for escalation, or None if already at top.
pub fn next_tier(current: &str) -> Option<&'static str> {
    match current {
        "SIMPLE" => Some("MEDIUM"),
        "MEDIUM" => Some("COMPLEX"),
        "COMPLEX" => Some("REASONING"),
        "REASONING" => None,
        _ => None,
    }
}
