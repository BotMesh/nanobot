use std::collections::HashMap;

pub fn default_weights() -> HashMap<&'static str, f32> {
    let mut m = HashMap::new();
    m.insert("reasoning", 0.18);
    m.insert("code", 0.15);
    m.insert("simple", 0.12);
    m.insert("multistep", 0.12);
    m.insert("technical", 0.10);
    m.insert("token_count", 0.08);
    m.insert("creative", 0.05);
    m.insert("question", 0.05);
    m.insert("constraint", 0.04);
    m.insert("imperative", 0.03);
    m.insert("format", 0.03);
    m.insert("domain", 0.02);
    m.insert("reference", 0.02);
    m.insert("negation", 0.01);
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
