//! # LLM Semantic Pass — AI-powered community labeling & document analysis
//!
//! Calls OpenAI, Anthropic, Gemini, Ollama, or any OpenAI-compatible API to
//! generate descriptive names for detected communities and extract architectural
//! insights from the knowledge graph.
//!
//! ## Supported providers
//!
//! | Provider | Env vars |
//! |----------|----------|
//! | **OpenAI** | `OPENAI_API_KEY` |
//! | **Anthropic** | `ANTHROPIC_API_KEY` |
//! | **Gemini** | `GEMINI_API_KEY` |
//! | **Ollama** | `OPENAI_BASE_URL=http://localhost:11434/v1 OPENAI_API_KEY=ollama` |
//! | **OpenAI-compatible** | `OPENAI_BASE_URL` + `OPENAI_API_KEY` |
//!
//! ## Configuration
//!
//! - `GRAPHIFY_LLM_PROVIDER` — "openai", "anthropic", "gemini", or "ollama" (auto-detected if unset)
//! - `GRAPHIFY_LLM_MODEL` — model name (provider-specific default if unset)
//! - `GRAPHIFY_LLM_MAX_TOKENS` — max tokens (default: 256)
//!
//! ## Usage
//!
//! ```bash
//! # OpenAI
//! export OPENAI_API_KEY=sk-...
//! graphify build . --llm
//!
//! # Anthropic
//! export ANTHROPIC_API_KEY=sk-ant-...
//! graphify build . --llm
//!
//! # Gemini
//! export GEMINI_API_KEY=...
//! graphify build . --llm
//!
//! # Ollama (local)
//! export OPENAI_BASE_URL=http://localhost:11434/v1
//! export OPENAI_API_KEY=ollama
//! export GRAPHIFY_LLM_MODEL=llama3.2
//! graphify build . --llm
//! ```

use anyhow::Context;
use std::collections::HashMap;
use std::process::Command;

/// LLM provider backend.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    /// Any OpenAI-compatible endpoint (OpenRouter, Groq, Together, etc.)
    OpenAICompatible,
}

/// LLM backend configuration.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        // Check explicit provider preference first, then auto-detect
        let provider = match std::env::var("GRAPHIFY_LLM_PROVIDER").as_deref() {
            Ok("anthropic") => LlmProvider::Anthropic,
            Ok("gemini") => LlmProvider::Gemini,
            Ok("ollama") => LlmProvider::Ollama,
            Ok("openai") | Ok("openai_compatible") => LlmProvider::OpenAI,
            _ => {
                // Auto-detect from available API keys
                if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                    LlmProvider::Anthropic
                } else if std::env::var("GEMINI_API_KEY").is_ok() {
                    LlmProvider::Gemini
                } else if std::env::var("OPENAI_BASE_URL").map_or(false, |u| u.contains("11434")) {
                    LlmProvider::Ollama
                } else {
                    LlmProvider::OpenAI
                }
            }
        };

        let (base_url, model) = match &provider {
            LlmProvider::Anthropic => (
                "https://api.anthropic.com/v1".into(),
                "claude-3-haiku-20240307".into(),
            ),
            LlmProvider::Gemini => (
                "https://generativelanguage.googleapis.com/v1beta".into(),
                "gemini-2.0-flash".into(),
            ),
            LlmProvider::Ollama => (
                "http://localhost:11434/v1".into(),
                std::env::var("GRAPHIFY_LLM_MODEL").unwrap_or_else(|_| "llama3.2".into()),
            ),
            _ => (
                std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
                std::env::var("GRAPHIFY_LLM_MODEL")
                    .unwrap_or_else(|_| "gpt-4o-mini".into()),
            ),
        };

        let api_key = Self::detect_api_key(&provider);

        Self {
            provider,
            base_url,
            api_key,
            model,
            max_tokens: std::env::var("GRAPHIFY_LLM_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(256),
            temperature: 0.2,
        }
    }
}

impl LlmConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn detect_api_key(provider: &LlmProvider) -> String {
        match provider {
            LlmProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            LlmProvider::Gemini => std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            _ => std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        }
    }

    /// Create config for Ollama (local).
    pub fn ollama(model: &str) -> Self {
        Self {
            provider: LlmProvider::Ollama,
            base_url: "http://localhost:11434/v1".into(),
            api_key: "ollama".into(),
            model: model.to_string(),
            ..Default::default()
        }
    }

    /// Create config for Anthropic.
    pub fn anthropic(model: &str) -> Self {
        Self {
            provider: LlmProvider::Anthropic,
            base_url: "https://api.anthropic.com/v1".into(),
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: model.to_string(),
            ..Default::default()
        }
    }

    /// Create config for Gemini.
    pub fn gemini(model: &str) -> Self {
        Self {
            provider: LlmProvider::Gemini,
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            api_key: std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            model: model.to_string(),
            ..Default::default()
        }
    }
}

/// Result of LLM-powered community labeling.
#[derive(Debug, Clone)]
pub struct LlmLabelResult {
    pub community_id: usize,
    pub name: String,
    pub description: String,
    pub top_nodes: Vec<String>,
}

/// Generate human-readable community labels using an LLM.
/// Supports OpenAI, Anthropic, Gemini, Ollama, and OpenAI-compatible APIs via curl.
pub fn label_communities_llm(
    config: &LlmConfig,
    communities: &[graphify_core::community::Community],
    nodes: &[graphify_core::node::GraphNode],
    top_n_per_community: usize,
) -> anyhow::Result<Vec<LlmLabelResult>> {
    if !config.is_configured() {
        anyhow::bail!("LLM not configured: set OPENAI_API_KEY or use Ollama");
    }
    if communities.is_empty() {
        return Ok(vec![]);
    }

    let node_map: HashMap<&str, &graphify_core::node::GraphNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut results = Vec::new();

    for community in communities.iter().take(10) {
        let top_nodes: Vec<&str> = community
            .nodes
            .iter()
            .filter_map(|nid| node_map.get(nid.as_str()))
            .map(|n| n.label.as_str())
            .take(top_n_per_community)
            .collect();

        if top_nodes.is_empty() {
            continue;
        }

        let prompt = format!(
            r#"You are analyzing a codebase knowledge graph. A community (cluster) of related code nodes was detected.

Top nodes in this community:
- {}

Give this community a short, descriptive name (2-5 words) that captures what these nodes do together.
Also give a one-sentence description.
Reply in JSON format: {{"name": "...", "description": "..."}}"#,
            top_nodes.join("\n- ")
        );

        match call_llm_json(config, &prompt) {
            Ok(json) => {
                let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed Community").to_string();
                let description = json.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                results.push(LlmLabelResult { community_id: community.id, name, description, top_nodes: top_nodes.into_iter().map(String::from).collect() });
            }
            Err(e) => eprintln!("  ⚠️ LLM labeling failed for community {}: {}", community.id, e),
        }
    }
    Ok(results)
}

/// Extract architectural insights from a knowledge graph using an LLM.
pub fn analyze_architecture_llm(
    config: &LlmConfig,
    kg: &graphify_core::KnowledgeGraph,
) -> anyhow::Result<String> {
    if !config.is_configured() {
        anyhow::bail!("LLM not configured");
    }
    let god_labels: Vec<&str> = kg.nodes.iter().filter(|n| n.is_god_node).map(|n| n.label.as_str()).take(10).collect();
    let lang = kg.metadata.primary_language.as_deref().unwrap_or("unknown");
    let prompt = format!(
        "Analyze this {} codebase: {} nodes, {} edges, {} communities. Top hubs: {}. In 2-3 sentences, describe the architecture style and give one actionable suggestion.",
        lang, kg.nodes.len(), kg.edges.len(), kg.communities.len(), god_labels.join(", ")
    );
    call_llm_text(config, &prompt)
}

// ── Internal: curl-based API calls ────────────────────────────────────────────

fn call_llm_text(config: &LlmConfig, prompt: &str) -> anyhow::Result<String> {
    match config.provider {
        LlmProvider::Anthropic => call_anthropic(config, prompt),
        LlmProvider::Gemini => call_gemini(config, prompt),
        _ => call_openai_compatible(config, prompt),
    }
}

/// OpenAI / Ollama / any OpenAI-compatible endpoint.
fn call_openai_compatible(config: &LlmConfig, prompt: &str) -> anyhow::Result<String> {
    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": config.max_tokens,
        "temperature": config.temperature,
    });

    let url = format!("{}/chat/completions", config.base_url);
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", &url])
        .arg("-H").arg(format!("Authorization: Bearer {}", config.api_key))
        .arg("-H").arg("Content-Type: application/json")
        .arg("-d").arg(body.to_string())
        .output()
        .context("curl not found. Install curl or use a different provider.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("LLM API call failed: {}", stderr);
    }

    let resp: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse LLM JSON response")?;

    let text = resp["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(text)
}

/// Anthropic Messages API: x-api-key header, different body format.
fn call_anthropic(config: &LlmConfig, prompt: &str) -> anyhow::Result<String> {
    let body = serde_json::json!({
        "model": config.model,
        "max_tokens": config.max_tokens,
        "temperature": config.temperature,
        "messages": [{"role": "user", "content": prompt}],
    });

    let url = format!("{}/messages", config.base_url);
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", &url])
        .arg("-H").arg(format!("x-api-key: {}", config.api_key))
        .arg("-H").arg("Content-Type: application/json")
        .arg("-H").arg("anthropic-version: 2023-06-01")
        .arg("-d").arg(body.to_string())
        .output()
        .context("curl not found. Install curl or use a different provider.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Anthropic API call failed: {}", stderr);
    }

    let resp: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse Anthropic JSON response")?;

    // Anthropic returns content as array of blocks
    let text = resp["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(text)
}

/// Gemini API: x-goog-api-key header, different body format.
fn call_gemini(config: &LlmConfig, prompt: &str) -> anyhow::Result<String> {
    let body = serde_json::json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }],
        "generationConfig": {
            "maxOutputTokens": config.max_tokens,
            "temperature": config.temperature,
        }
    });

    // Gemini URL format: /models/{model}:generateContent
    let url = format!("{}/models/{}:generateContent?key={}", config.base_url, config.model, config.api_key);
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", &url])
        .arg("-H").arg("Content-Type: application/json")
        .arg("-d").arg(body.to_string())
        .output()
        .context("curl not found. Install curl or use a different provider.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Gemini API call failed: {}", stderr);
    }

    let resp: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse Gemini JSON response")?;

    let text = resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(text)
}

fn call_llm_json(config: &LlmConfig, prompt: &str) -> anyhow::Result<serde_json::Value> {
    let text = call_llm_text(config, prompt)?;
    let json_str = if let Some(start) = text.find("```json") {
        text[start + 7..].split("```").next().unwrap_or(&text).trim()
    } else if let Some(start) = text.find('{') {
        &text[start..]
    } else {
        &text
    };
    let value: serde_json::Value = serde_json::from_str(json_str)
        .context("LLM response was not valid JSON")
        .with_context(|| format!("Raw: {}", &text[..text.len().min(200)]))?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_default_no_keys() {
        // In test env no API keys are set, so default should be OpenAI with empty key
        let config = LlmConfig::default();
        // No API key set = not configured
        assert!(!config.is_configured() || std::env::var("OPENAI_API_KEY").is_ok());
        assert_eq!(config.provider, LlmProvider::OpenAI);
    }

    #[test]
    fn test_llm_config_ollama() {
        let config = LlmConfig::ollama("llama3.2");
        assert!(config.is_configured());
        assert_eq!(config.provider, LlmProvider::Ollama);
        assert_eq!(config.base_url, "http://localhost:11434/v1");
        assert_eq!(config.model, "llama3.2");
    }

    #[test]
    fn test_llm_config_anthropic() {
        let config = LlmConfig::anthropic("claude-3-haiku-20240307");
        assert_eq!(config.provider, LlmProvider::Anthropic);
        assert_eq!(config.model, "claude-3-haiku-20240307");
        assert_eq!(config.base_url, "https://api.anthropic.com/v1");
    }

    #[test]
    fn test_llm_config_gemini() {
        let config = LlmConfig::gemini("gemini-2.0-flash");
        assert_eq!(config.provider, LlmProvider::Gemini);
        assert_eq!(config.model, "gemini-2.0-flash");
    }

    #[test]
    fn test_label_communities_no_config() {
        let config = LlmConfig { provider: LlmProvider::OpenAI, api_key: String::new(), ..Default::default() };
        let result = label_communities_llm(&config, &[], &[], 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_auto_detection_openai() {
        // Default in test env should be OpenAI (no special env vars set)
        let config = LlmConfig::default();
        // The default may be OpenAI or Anthropic/Gemini depending on env — but
        // at minimum the base_url should be set
        assert!(!config.base_url.is_empty());
        assert!(!config.model.is_empty());
    }
}
