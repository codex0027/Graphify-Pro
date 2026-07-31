//! Community (subsystem) detection results.

use serde::{Deserialize, Serialize};

/// A detected community (subsystem) within the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// Unique community ID
    pub id: usize,
    /// Human-readable label (may be LLM-generated)
    pub label: String,
    /// Node IDs belonging to this community
    pub nodes: Vec<String>,
    /// Modularity score contribution
    pub modularity: f64,
    /// Size of the community
    pub size: usize,
    /// Core "hub" nodes within this community
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hubs: Vec<String>,
    /// Parent community (for hierarchical clustering)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<usize>,
    /// Whether this community was labeled by an LLM
    #[serde(default)]
    pub llm_labeled: bool,
    /// Metadata about this community
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
