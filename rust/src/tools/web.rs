//! Web tools: web_search and web_fetch.

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

use super::base::{object_schema, string_prop, Tool};

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7_2) AppleWebKit/537.36";
const MAX_REDIRECTS: usize = 5;

/// Strip HTML tags and decode entities.
fn strip_tags(text: &str) -> String {
    // Remove script tags
    let re_script = Regex::new(r"(?is)<script[\s\S]*?</script>").unwrap();
    let text = re_script.replace_all(text, "");

    // Remove style tags
    let re_style = Regex::new(r"(?is)<style[\s\S]*?</style>").unwrap();
    let text = re_style.replace_all(&text, "");

    // Remove all other tags
    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    let text = re_tags.replace_all(&text, "");

    // Decode HTML entities
    html_escape::decode_html_entities(&text).to_string()
}

/// Normalize whitespace.
fn normalize(text: &str) -> String {
    let re_spaces = Regex::new(r"[ \t]+").unwrap();
    let text = re_spaces.replace_all(text, " ");

    let re_newlines = Regex::new(r"\n{3,}").unwrap();
    re_newlines.replace_all(&text, "\n\n").trim().to_string()
}

/// Validate URL: must be http(s) with valid domain.
fn validate_url(url_str: &str) -> Result<Url, String> {
    let url = Url::parse(url_str).map_err(|e| e.to_string())?;

    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(format!("Only http/https allowed, got '{}'", scheme)),
    }

    if url.host().is_none() {
        return Err("Missing domain".to_string());
    }

    Ok(url)
}

/// Convert HTML to markdown.
fn html_to_markdown(html: &str) -> String {
    let mut text = html.to_string();

    // Convert links: <a href="url">text</a> -> [text](url)
    let re_links =
        Regex::new(r#"(?is)<a\s+[^>]*href=["']([^"']+)["'][^>]*>([\s\S]*?)</a>"#).unwrap();
    text = re_links
        .replace_all(&text, |caps: &regex::Captures| {
            let url = &caps[1];
            let link_text = strip_tags(&caps[2]);
            format!("[{}]({})", link_text, url)
        })
        .to_string();

    // Convert headings: <h1>text</h1> -> # text (handle each level separately)
    for level in 1..=6 {
        let pattern = format!(r"(?is)<h{}[^>]*>([\s\S]*?)</h{}>", level, level);
        let re_heading = Regex::new(&pattern).unwrap();
        text = re_heading
            .replace_all(&text, |caps: &regex::Captures| {
                let heading_text = strip_tags(&caps[1]);
                format!("\n{} {}\n", "#".repeat(level), heading_text)
            })
            .to_string();
    }

    // Convert list items: <li>text</li> -> - text
    let re_li = Regex::new(r"(?is)<li[^>]*>([\s\S]*?)</li>").unwrap();
    text = re_li
        .replace_all(&text, |caps: &regex::Captures| {
            format!("\n- {}", strip_tags(&caps[1]))
        })
        .to_string();

    // Block element endings -> newlines
    let re_blocks = Regex::new(r"(?i)</(p|div|section|article)>").unwrap();
    text = re_blocks.replace_all(&text, "\n\n").to_string();

    // Line breaks
    let re_br = Regex::new(r"(?i)<(br|hr)\s*/?>").unwrap();
    text = re_br.replace_all(&text, "\n").to_string();

    normalize(&strip_tags(&text))
}

/// Search the web using Brave Search API.
#[pyclass]
#[derive(Clone)]
pub struct WebSearchTool {
    api_key: String,
    max_results: usize,
}

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web. Returns titles, URLs, and snippets."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("query".into(), string_prop("Search query"));
        props.insert(
            "count".into(),
            json!({
                "type": "integer",
                "description": "Results (1-10)",
                "minimum": 1,
                "maximum": 10
            }),
        );
        object_schema(props, vec!["query"])
    }
}

#[pymethods]
impl WebSearchTool {
    #[new]
    #[pyo3(signature = (api_key=None, max_results=5))]
    fn new(api_key: Option<String>, max_results: usize) -> Self {
        let key = api_key.unwrap_or_else(|| std::env::var("BRAVE_API_KEY").unwrap_or_default());
        Self {
            api_key: key,
            max_results,
        }
    }

    #[getter]
    fn name(&self) -> &str {
        "web_search"
    }

    #[getter]
    fn description(&self) -> &str {
        Tool::description(self)
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        let params = Tool::parameters(self);
        let json_str = serde_json::to_string(&params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let result = py.import("json")?.call_method1("loads", (json_str,))?;
        Ok(result.into())
    }

    #[pyo3(signature = (query, count=None))]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        query: String,
        count: Option<usize>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let api_key = self.api_key.clone();
        let max_results = self.max_results;

        future_into_py(py, async move {
            if api_key.is_empty() {
                return Ok("Error: BRAVE_API_KEY not configured".to_string());
            }

            let n = count.unwrap_or(max_results).clamp(1, 10);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            let resp = client
                .get("https://api.search.brave.com/res/v1/web/search")
                .query(&[("q", &query), ("count", &n.to_string())])
                .header("Accept", "application/json")
                .header("X-Subscription-Token", &api_key)
                .send()
                .await;

            match resp {
                Ok(r) => {
                    if !r.status().is_success() {
                        return Ok(format!("Error: HTTP {}", r.status()));
                    }

                    let data: serde_json::Value = r
                        .json()
                        .await
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

                    let results = data
                        .get("web")
                        .and_then(|w| w.get("results"))
                        .and_then(|r| r.as_array());

                    match results {
                        Some(items) if !items.is_empty() => {
                            let mut lines = vec![format!("Results for: {}\n", query)];
                            for (i, item) in items.iter().take(n).enumerate() {
                                let title =
                                    item.get("title").and_then(|t| t.as_str()).unwrap_or("");
                                let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
                                lines.push(format!("{}. {}\n   {}", i + 1, title, url));
                                if let Some(desc) = item.get("description").and_then(|d| d.as_str())
                                {
                                    lines.push(format!("   {}", desc));
                                }
                            }
                            Ok(lines.join("\n"))
                        }
                        _ => Ok(format!("No results for: {}", query)),
                    }
                }
                Err(e) => Ok(format!("Error: {}", e)),
            }
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}

/// Fetch and extract content from a URL.
#[pyclass]
#[derive(Clone)]
pub struct WebFetchTool {
    max_chars: usize,
}

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch URL and extract readable content (HTML → markdown/text)."
    }

    fn parameters(&self) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("url".into(), string_prop("URL to fetch"));
        props.insert(
            "extractMode".into(),
            json!({
                "type": "string",
                "enum": ["markdown", "text"],
                "default": "markdown"
            }),
        );
        props.insert(
            "maxChars".into(),
            json!({
                "type": "integer",
                "minimum": 100
            }),
        );
        object_schema(props, vec!["url"])
    }
}

#[pymethods]
impl WebFetchTool {
    #[new]
    #[pyo3(signature = (max_chars=50000))]
    fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }

    #[getter]
    fn name(&self) -> &str {
        "web_fetch"
    }

    #[getter]
    fn description(&self) -> &str {
        Tool::description(self)
    }

    #[getter]
    fn parameters(&self, py: Python<'_>) -> PyResult<PyObject> {
        let params = Tool::parameters(self);
        let json_str = serde_json::to_string(&params)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let result = py.import("json")?.call_method1("loads", (json_str,))?;
        Ok(result.into())
    }

    #[pyo3(signature = (url, extractMode="markdown", maxChars=None))]
    #[allow(non_snake_case)]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        url: String,
        extractMode: &str,
        maxChars: Option<usize>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let max_chars = maxChars.unwrap_or(self.max_chars);
        let extract_mode = extractMode.to_string();

        future_into_py(py, async move {
            // Validate URL
            let parsed_url = match validate_url(&url) {
                Ok(u) => u,
                Err(e) => {
                    return Ok(json!({
                        "error": format!("URL validation failed: {}", e),
                        "url": url
                    })
                    .to_string());
                }
            };

            let client = reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            let resp = client.get(parsed_url.as_str()).send().await;

            match resp {
                Ok(r) => {
                    let status = r.status().as_u16();
                    let final_url = r.url().to_string();
                    let content_type = r
                        .headers()
                        .get("content-type")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("")
                        .to_string();

                    let body = r
                        .text()
                        .await
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

                    let (text, extractor) = if content_type.contains("application/json") {
                        // JSON - pretty print
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(v) => (serde_json::to_string_pretty(&v).unwrap_or(body), "json"),
                            Err(_) => (body, "raw"),
                        }
                    } else if content_type.contains("text/html")
                        || body.trim_start()[..256.min(body.len())]
                            .to_lowercase()
                            .starts_with("<!doctype")
                        || body.trim_start()[..256.min(body.len())]
                            .to_lowercase()
                            .starts_with("<html")
                    {
                        // HTML - extract content
                        let content = if extract_mode == "markdown" {
                            html_to_markdown(&body)
                        } else {
                            strip_tags(&body)
                        };

                        // Try to extract title
                        let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
                        let title = title_re
                            .captures(&body)
                            .map(|c| strip_tags(&c[1]))
                            .unwrap_or_default();

                        let text = if !title.is_empty() {
                            format!("# {}\n\n{}", title, content)
                        } else {
                            content
                        };

                        (text, "readability")
                    } else {
                        (body, "raw")
                    };

                    let truncated = text.len() > max_chars;
                    let text = if truncated {
                        text[..max_chars].to_string()
                    } else {
                        text
                    };

                    Ok(json!({
                        "url": url,
                        "finalUrl": final_url,
                        "status": status,
                        "extractor": extractor,
                        "truncated": truncated,
                        "length": text.len(),
                        "text": text
                    })
                    .to_string())
                }
                Err(e) => Ok(json!({
                    "error": e.to_string(),
                    "url": url
                })
                .to_string()),
            }
        })
    }

    fn to_schema_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        let schema = Tool::to_schema(self, py)?;
        schema.to_dict(py)
    }
}
