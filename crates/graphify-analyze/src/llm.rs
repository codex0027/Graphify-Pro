//! # LLM Semantic Pass — AI-powered community labeling & document analysis
//!
//! Calls OpenAI-compatible APIs to generate descriptive names for detected
//! communities and extract architectural insights from the knowledge graph.
//!
//! ## Configuration
//!
//! Set environment variables:
//! - `OPENAI_API_KEY` — API key (required)
//! - `OPENAI_BASE_URL` — custom endpoint (default: https://api.openai.com/v1)
//!   Use `http://localhost:11434/v1` for Ollama, or any OpenAI-compatible endpoint
//! - `GRAPHIFY_LLM_MODEL` — model name (default: gpt-4o-mini)
//!
//! ## Usage
//!
//! ```bash
//! # OpenAI
//! export OPENAI_API_KEY=sk-...
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

/// LLM backend configuration.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: std::env::var("GRAPHIFY_LLM_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".into()),
            max_tokens: 256,
            temperature: 0.2,
        }
    }
}

impl LlmConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub fn ollama(model: &str) -> Self {
        Self {
            base_url: "http://localhost:11434/v1".into(),
            api_key: "ollama".into(),
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
/// Uses `curl` to call OpenAI-compatible APIs.
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
        .context("curl not found. Install curl or set OPENAI_API_KEY.")?;

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
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        // No API key set = not configured
        assert!(!config.is_configured() || std::env::var("OPENAI_API_KEY").is_ok());
    }

    #[test]
    fn test_llm_config_ollama() {
        let config = LlmConfig::ollama("llama3.2");
        assert!(config.is_configured());
        assert_eq!(config.base_url, "http://localhost:11434/v1");
    }

    #[test]
    fn test_label_communities_no_config() {
        let config = LlmConfig { api_key: String::new(), ..Default::default() };
        let result = label_communities_llm(&config, &[], &[], 5);
        assert!(result.is_err());
    }
}
