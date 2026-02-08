use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn score_text(text: &str) -> HashMap<&'static str, f32> {
    let mut scores = HashMap::new();

    let lower = text.to_lowercase();

    // Simple keyword checks
    let reasoning_keywords = ["prove", "theorem", "step by step", "formal"];
    let code_keywords = ["function", "import", "async", "```", "class"];

    let mut reasoning = 0.0f32;
    for k in reasoning_keywords.iter() {
        if lower.contains(k) {
            reasoning += 1.0;
        }
    }

    let mut code = 0.0f32;
    for k in code_keywords.iter() {
        if lower.contains(k) {
            code += 1.0;
        }
    }

    // Multi-step patterns
    static MULTISTEP_RE: OnceLock<Regex> = OnceLock::new();
    let multistep_re = MULTISTEP_RE.get_or_init(|| Regex::new(r"first\b|then\b|step \d").unwrap());
    let multistep = if multistep_re.is_match(&lower) {
        1.0
    } else {
        0.0
    };

    // Token/length proxy
    let len = lower.len() as f32;
    let token_count = if len < 100.0 {
        0.2
    } else if len > 1000.0 {
        1.0
    } else {
        (len - 100.0) / 900.0
    };

    // Questions
    let question = if lower.contains("?") { 1.0 } else { 0.0 };

    // Creative marker
    let creative =
        if lower.contains("story") || lower.contains("poem") || lower.contains("brainstorm") {
            1.0
        } else {
            0.0
        };

    // Imperative verbs
    let imperative =
        if lower.contains("build") || lower.contains("create") || lower.contains("implement") {
            1.0
        } else {
            0.0
        };

    // Output format
    let format = if lower.contains("json") || lower.contains("yaml") || lower.contains("schema") {
        1.0
    } else {
        0.0
    };

    // Technical terms heuristic
    let technical = if lower.contains("kubernetes")
        || lower.contains("algorithm")
        || lower.contains("distributed")
    {
        1.0
    } else {
        0.0
    };

    // Negation
    let negation =
        if lower.contains("don't") || lower.contains("avoid") || lower.contains("without") {
            1.0
        } else {
            0.0
        };

    scores.insert("reasoning", reasoning.min(3.0) / 3.0);
    scores.insert("code", code.min(3.0) / 3.0);
    scores.insert("multistep", multistep);
    scores.insert("token_count", token_count);
    scores.insert("question", question);
    scores.insert("creative", creative);
    scores.insert("imperative", imperative);
    scores.insert("format", format);
    scores.insert("technical", technical);
    scores.insert("negation", negation);

    scores
}
