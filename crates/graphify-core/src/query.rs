//! Graph query types for graph traversal and search.

use serde::{Deserialize, Serialize};

/// A query against the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Natural language question or structured query
    pub question: String,
    /// Search strategy
    #[serde(default)]
    pub strategy: QueryStrategy,
    /// Maximum token budget for results
    #[serde(default = "default_budget")]
    pub budget: usize,
    /// Maximum traversal depth
    #[serde(default = "default_depth")]
    pub max_depth: usize,
    /// Filter by edge relation types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relation_filter: Vec<String>,
    /// Filter by node types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_type_filter: Vec<String>,
    /// Filter by confidence minimum
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<f64>,
    /// Exclude specific node types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_types: Vec<String>,
}

fn default_budget() -> usize {
    2000
}

fn default_depth() -> usize {
    3
}

/// Traversal strategy for graph queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueryStrategy {
    /// Breadth-first search (good for exploring nearby connections)
    #[default]
    BreadthFirst,
    /// Depth-first search (good for tracing long paths)
    DepthFirst,
    /// Shortest path between specific nodes
    ShortestPath,
    /// Reverse traversal (find what depends on X)
    ReverseDependency,
    /// Community-focused search
    CommunityScope,
}

/// Result of a graph query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// The original query
    pub query: String,
    /// Nodes matching the query
    pub nodes: Vec<QueryMatch>,
    /// Sub-graph edges connecting matched nodes
    pub edges: Vec<super::edge::GraphEdge>,
    /// Estimated token count of the result
    pub token_count: usize,
    /// Whether the budget was exceeded
    pub budget_exceeded: bool,
}

/// A node match from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMatch {
    /// Node ID
    pub node_id: String,
    /// Node label
    pub label: String,
    /// Relevance score
    pub score: f64,
    /// Path from query root to this node
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    /// Depth from query root
    pub depth: usize,
}

impl Default for GraphQuery {
    fn default() -> Self {
        Self {
            question: String::new(),
            strategy: QueryStrategy::default(),
            budget: default_budget(),
            max_depth: default_depth(),
            relation_filter: Vec::new(),
            node_type_filter: Vec::new(),
            min_confidence: None,
            exclude_types: Vec::new(),
        }
    }
}
