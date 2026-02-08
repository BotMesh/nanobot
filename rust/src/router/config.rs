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
    let idx = TIER_ORDER.iter().position(|t| *t == current)?;
    TIER_ORDER.get(idx + 1).copied()
}

/// Alternative models per tier, sorted by cost ascending (cheapest first).
/// Includes models from multiple providers for cross-provider billing fallback.
pub fn tier_alternatives() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();
    m.insert(
        "SIMPLE",
        vec![
            "groq/llama-3.3-70b-versatile", // free tier
            "deepseek/deepseek-chat",       // $0.42
            "openai/gpt-4o-mini",           // $0.60
            "openai/gpt-3.5-turbo",         // $1.50
        ],
    );
    m.insert(
        "MEDIUM",
        vec![
            "groq/llama-3.3-70b-versatile", // free tier
            "deepseek/deepseek-chat",       // $0.42
            "openai/gpt-4o-mini",           // $0.60
            "minimax/minimax-m2",           // $1.20
        ],
    );
    m.insert(
        "COMPLEX",
        vec![
            "groq/llama-3.3-70b-versatile", // free tier (best-effort)
            "anthropic/claude-sonnet-4-5",  // $15.00
            "openai/gpt-4o",                // $10.00
            "anthropic/claude-opus-4-5",    // $25.00
        ],
    );
    m.insert(
        "REASONING",
        vec![
            "groq/llama-3.3-70b-versatile", // free tier (best-effort)
            "deepseek/deepseek-reasoner",   // $2.19
            "openai/o3-mini",               // $4.40
            "openai/o3",                    // $8.00
        ],
    );
    m
}
