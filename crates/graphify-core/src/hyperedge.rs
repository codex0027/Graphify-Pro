//! Hyper-edge types — connecting 3+ nodes sharing a concept.

use serde::{Deserialize, Serialize};
use crate::confidence::Confidence;

/// A hyper-edge connecting 3+ nodes around a shared concept or pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperEdge {
    /// Unique identifier
    pub id: String,
    /// Shared concept label
    pub label: String,
    /// Nodes participating in this hyper-edge
    pub nodes: Vec<String>,
    /// Source file (if applicable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    /// Confidence level
    #[serde(default)]
    pub confidence: Confidence,
    /// Weight
    #[serde(default = "super::edge::default_weight")]
    pub weight: f64,
}

impl HyperEdge {
    pub fn new(id: impl Into<String>, label: impl Into<String>, nodes: Vec<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            nodes,
            source_file: None,
            confidence: Confidence::Inferred,
            weight: 1.0,
        }
    }
}
