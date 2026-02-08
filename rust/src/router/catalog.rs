use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
    pricing: Option<ModelPricing>,
    context_length: Option<u64>,
}

#[derive(Deserialize)]
struct ModelPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

/// Cached catalog fetched once from OpenRouter (pricing + context lengths).
struct Catalog {
    pricing: HashMap<&'static str, f64>,
    context_lengths: HashMap<&'static str, u64>,
}

fn get_catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut pricing = HashMap::new();
        let mut context_lengths = HashMap::new();

        // Pull all models from OpenRouter so the catalog stays current.
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
        {
            if let Ok(resp) = client.get("https://openrouter.ai/api/v1/models").send() {
                if let Ok(payload) = resp.json::<ModelsResponse>() {
                    for entry in payload.data {
                        let key: &'static str = Box::leak(entry.id.into_boxed_str());

                        // Context length
                        if let Some(ctx) = entry.context_length {
                            context_lengths.insert(key, ctx);
                        }

                        // Pricing (USD per 1M output tokens)
                        if let Some(p) = entry.pricing {
                            if let Some(completion) = p.completion {
                                if let Ok(price_per_token) = completion.parse::<f64>() {
                                    pricing.insert(key, price_per_token * 1_000_000.0);
                                }
                            } else if let Some(prompt) = p.prompt {
                                if let Ok(price_per_token) = prompt.parse::<f64>() {
                                    pricing.insert(key, price_per_token * 1_000_000.0);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Official provider pricing overrides (USD per 1M output tokens).
        let price_overrides: [(&'static str, f64); 6] = [
            ("openai/gpt-3.5-turbo", 1.50),
            ("openai/gpt-4o-mini", 0.60),
            ("openai/o3", 8.00),
            ("anthropic/claude-opus-4-5", 25.00),
            ("deepseek/deepseek-chat", 0.42),
            ("minimax/minimax-m2", 1.20),
        ];
        for (model, price) in price_overrides {
            pricing.insert(model, price);
        }

        // Context length overrides for core tier models (guaranteed fallback).
        let ctx_overrides: [(&'static str, u64); 6] = [
            ("openai/gpt-3.5-turbo", 16_384),
            ("openai/gpt-4o-mini", 128_000),
            ("anthropic/claude-opus-4-5", 200_000),
            ("openai/o3", 200_000),
            ("deepseek/deepseek-chat", 128_000),
            ("minimax/minimax-m2", 1_000_000),
        ];
        for (model, ctx) in ctx_overrides {
            context_lengths.insert(model, ctx);
        }

        // README-referenced models to ensure a non-empty fallback when network is unavailable.
        pricing
            .entry("meta-llama/Llama-3.1-8B-Instruct")
            .or_insert(0.0);
        context_lengths
            .entry("meta-llama/Llama-3.1-8B-Instruct")
            .or_insert(131_072);

        Catalog {
            pricing,
            context_lengths,
        }
    })
}

pub fn default_pricing() -> HashMap<&'static str, f64> {
    get_catalog().pricing.clone()
}

pub fn default_context_lengths() -> HashMap<&'static str, u64> {
    get_catalog().context_lengths.clone()
}
