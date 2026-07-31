//! Confidence tagging for graph edges.
//!
//! Every relationship edge is explicitly tagged:
//! - `EXTRACTED` — found deterministically in source code (imports, direct calls).
//! - `INFERRED` — deduced through semantic analysis or cross-references.
//! - `AMBIGUOUS` — possible but uncertain connection.

use serde::{Deserialize, Serialize};

/// Confidence level for graph relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Confidence {
    /// Deterministically extracted from source (e.g., AST-parsed imports, direct calls)
    Extracted,
    /// Deduced through semantic analysis or second-pass cross-references
    Inferred,
    /// Possible connection flagged as uncertain
    Ambiguous,
}

impl Confidence {
    /// Returns a numeric score for weighting algorithms (0.0 - 1.0).
    pub fn score(&self) -> f64 {
        match self {
            Confidence::Extracted => 1.0,
            Confidence::Inferred => 0.7,
            Confidence::Ambiguous => 0.3,
        }
    }

    /// Whether this confidence level is considered "reliable" for analysis.
    pub fn is_reliable(&self) -> bool {
        matches!(self, Confidence::Extracted | Confidence::Inferred)
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Confidence::Extracted => "EXTRACTED",
            Confidence::Inferred => "INFERRED",
            Confidence::Ambiguous => "AMBIGUOUS",
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Confidence::Extracted
    }
}

/// A confidence score with optional reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceAnnotation {
    pub level: Confidence,
    /// Optional numeric score for finer granularity
    pub score: Option<f64>,
    /// Human-readable justification
    pub reasoning: Option<String>,
}

impl ConfidenceAnnotation {
    pub fn extracted() -> Self {
        Self {
            level: Confidence::Extracted,
            score: Some(1.0),
            reasoning: None,
        }
    }

    pub fn inferred(reasoning: impl Into<String>) -> Self {
        Self {
            level: Confidence::Inferred,
            score: Some(0.7),
            reasoning: Some(reasoning.into()),
        }
    }

    pub fn ambiguous(reasoning: impl Into<String>) -> Self {
        Self {
            level: Confidence::Ambiguous,
            score: Some(0.3),
            reasoning: Some(reasoning.into()),
        }
    }
}
