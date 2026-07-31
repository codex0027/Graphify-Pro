//! Graph node types for Graphify Pro.

use serde::{Deserialize, Serialize};
use crate::confidence::Confidence;

/// A node in the knowledge graph, representing a code entity, file, concept, or artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier for this node
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Type of node
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Source file this node was extracted from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    /// Line/column location in source
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<String>,
    /// Confidence of extraction
    #[serde(default)]
    pub confidence: Confidence,
    /// Whether this is a "god node" (highly connected hub)
    #[serde(default)]
    pub is_god_node: bool,
    /// Community ID this node belongs to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_id: Option<usize>,
    /// Arbitrary metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Language of the source file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// Classification of node types in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// A source code file
    File,
    /// A class definition
    Class,
    /// A function or method definition
    Function,
    /// A variable or constant
    Variable,
    /// An interface or trait
    Interface,
    /// A module or namespace
    Module,
    /// An enum definition
    Enum,
    /// A type alias
    TypeAlias,
    /// A documentation artifact
    Document,
    /// A rationale comment or design note
    Rationale,
    /// An external dependency
    Dependency,
    /// A design decision reference (ADR, RFC)
    DesignDecision,
    /// A database table
    DatabaseTable,
    /// An API endpoint
    ApiEndpoint,
    /// A configuration artifact
    Config,
    /// A concept extracted semantically
    Concept,
    /// Unknown/custom type
    Other(String),
}

impl NodeType {
    /// Returns a display name for this node type.
    pub fn label(&self) -> &str {
        match self {
            NodeType::File => "file",
            NodeType::Class => "class",
            NodeType::Function => "function",
            NodeType::Variable => "variable",
            NodeType::Interface => "interface",
            NodeType::Module => "module",
            NodeType::Enum => "enum",
            NodeType::TypeAlias => "type_alias",
            NodeType::Document => "document",
            NodeType::Rationale => "rationale",
            NodeType::Dependency => "dependency",
            NodeType::DesignDecision => "design_decision",
            NodeType::DatabaseTable => "database_table",
            NodeType::ApiEndpoint => "api_endpoint",
            NodeType::Config => "config",
            NodeType::Concept => "concept",
            NodeType::Other(_) => "other",
        }
    }

    /// Whether this node type represents a structural code element.
    pub fn is_code(&self) -> bool {
        matches!(
            self,
            NodeType::File
                | NodeType::Class
                | NodeType::Function
                | NodeType::Variable
                | NodeType::Interface
                | NodeType::Module
                | NodeType::Enum
                | NodeType::TypeAlias
        )
    }

    /// Whether this node type represents documentation/rationale.
    pub fn is_rationale(&self) -> bool {
        matches!(self, NodeType::Rationale | NodeType::DesignDecision | NodeType::Document)
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Other(s) => write!(f, "{}", s),
            _ => write!(f, "{}", self.label()),
        }
    }
}

impl GraphNode {
    /// Create a new graph node.
    pub fn new(id: impl Into<String>, label: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            node_type,
            source_file: None,
            source_location: None,
            confidence: Confidence::Extracted,
            is_god_node: false,
            community_id: None,
            metadata: None,
            language: None,
        }
    }

    /// Set the source location for this node.
    pub fn with_source(mut self, file: impl Into<String>, location: impl Into<String>) -> Self {
        self.source_file = Some(file.into());
        self.source_location = Some(location.into());
        self
    }

    /// Set the language.
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }
}
