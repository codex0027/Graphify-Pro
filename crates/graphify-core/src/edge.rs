//! Graph edge types for Graphify Pro.

use serde::{Deserialize, Serialize};
use crate::confidence::Confidence;

/// A directed edge connecting two nodes in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Type of relationship
    pub relation: EdgeRelation,
    /// Additional context about this edge
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Confidence level
    #[serde(default)]
    pub confidence: Confidence,
    /// Source file this edge was extracted from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    /// Line/column location in source
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<String>,
    /// Edge weight (for graph algorithms)
    #[serde(default = "default_weight")]
    pub weight: f64,
    /// Arbitrary metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

pub fn default_weight() -> f64 {
    1.0
}

/// Types of relationships between graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    /// A calls B (function/method invocation)
    Calls,
    /// A imports B (module import)
    Imports,
    /// A imports from B (named import)
    ImportsFrom,
    /// A inherits from B (class/interface inheritance)
    Inherits,
    /// A implements B (interface implementation)
    Implements,
    /// A contains B (membership/ownership)
    Contains,
    /// A references B (general reference)
    References,
    /// A exports B (re-export)
    ReExports,
    /// A mixes in B (trait/mixin)
    MixesIn,
    /// B is a method of A
    Method,
    /// A embeds B
    Embeds,
    /// Rationale or comment explaining A
    RationaleFor,
    /// A depends on B (generic dependency)
    DependsOn,
    /// A is related to B (weak semantic link)
    RelatedTo,
    /// Database foreign key relationship
    ForeignKey,
    /// HTTP/API call relationship
    HttpCalls,
    /// Event/message relationship
    Emits,
    /// Custom relation
    Custom(String),
}

impl EdgeRelation {
    pub fn label(&self) -> &str {
        match self {
            EdgeRelation::Calls => "calls",
            EdgeRelation::Imports => "imports",
            EdgeRelation::ImportsFrom => "imports_from",
            EdgeRelation::Inherits => "inherits",
            EdgeRelation::Implements => "implements",
            EdgeRelation::Contains => "contains",
            EdgeRelation::References => "references",
            EdgeRelation::ReExports => "re_exports",
            EdgeRelation::MixesIn => "mixes_in",
            EdgeRelation::Method => "method",
            EdgeRelation::Embeds => "embeds",
            EdgeRelation::RationaleFor => "rationale_for",
            EdgeRelation::DependsOn => "depends_on",
            EdgeRelation::RelatedTo => "related_to",
            EdgeRelation::ForeignKey => "foreign_key",
            EdgeRelation::HttpCalls => "http_calls",
            EdgeRelation::Emits => "emits",
            EdgeRelation::Custom(_) => "custom",
        }
    }

    /// Whether this is a structural (code-level) relationship.
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            EdgeRelation::Calls
                | EdgeRelation::Imports
                | EdgeRelation::ImportsFrom
                | EdgeRelation::Inherits
                | EdgeRelation::Implements
                | EdgeRelation::Contains
                | EdgeRelation::Method
                | EdgeRelation::ReExports
        )
    }

    /// Whether this is a semantic (inferred) relationship.
    pub fn is_semantic(&self) -> bool {
        matches!(
            self,
            EdgeRelation::References | EdgeRelation::DependsOn | EdgeRelation::RelatedTo
        )
    }
}

impl std::fmt::Display for EdgeRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeRelation::Custom(s) => write!(f, "{}", s),
            _ => write!(f, "{}", self.label()),
        }
    }
}

impl GraphEdge {
    /// Create a new graph edge.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation: EdgeRelation,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation,
            context: None,
            confidence: Confidence::Extracted,
            source_file: None,
            source_location: None,
            weight: 1.0,
            metadata: None,
        }
    }

    /// Set the confidence level.
    pub fn with_confidence(mut self, confidence: Confidence, score: Option<f64>) -> Self {
        self.confidence = confidence;
        if let Some(s) = score {
            self.weight = s;
        }
        self
    }

    /// Set source location.
    pub fn with_source(mut self, file: impl Into<String>, location: impl Into<String>) -> Self {
        self.source_file = Some(file.into());
        self.source_location = Some(location.into());
        self
    }
}
