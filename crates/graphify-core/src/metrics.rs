//! Graph metrics and algorithms for code quality analysis.

use serde::{Deserialize, Serialize};

/// Metrics computed on a graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// Node ID
    pub node_id: String,
    /// In-degree (incoming edges)
    pub in_degree: usize,
    /// Out-degree (outgoing edges)
    pub out_degree: usize,
    /// Total degree
    pub total_degree: usize,
    /// Betweenness centrality
    pub betweenness: f64,
    /// Closeness centrality
    pub closeness: f64,
    /// PageRank score
    pub pagerank: f64,
    /// Eigenvector centrality
    pub eigenvector: f64,
    /// Whether this is a "god node"
    pub is_god_node: bool,
    /// Whether this is a "leaf" (no outgoing edges)
    pub is_leaf: bool,
    /// Clustering coefficient
    pub clustering_coefficient: f64,
}

/// Code quality issues detected via graph analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeQualityIssue {
    /// Type of issue
    pub issue_type: CodeQualityType,
    /// Node(s) involved
    pub nodes: Vec<String>,
    /// Severity (0.0 - 1.0)
    pub severity: f64,
    /// Human-readable description
    pub description: String,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Types of code quality issues detectable via graph analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeQualityType {
    /// A class/module with too many responsibilities
    GodClass,
    /// A function with too many dependencies
    GodFunction,
    /// Circular dependency cycle
    CircularDependency,
    /// Dead code (unreachable or unused)
    DeadCode,
    /// Architectural violation (dependency direction wrong)
    ArchitectureViolation,
    /// High coupling between modules
    HighCoupling,
    /// Low cohesion within a module
    LowCohesion,
    /// A file too large in terms of edges
    HubFile,
    /// Dependency on deprecated node
    DeprecatedDependency,
}

impl CodeQualityType {
    pub fn label(&self) -> &str {
        match self {
            CodeQualityType::GodClass => "God Class",
            CodeQualityType::GodFunction => "God Function",
            CodeQualityType::CircularDependency => "Circular Dependency",
            CodeQualityType::DeadCode => "Dead Code",
            CodeQualityType::ArchitectureViolation => "Architecture Violation",
            CodeQualityType::HighCoupling => "High Coupling",
            CodeQualityType::LowCohesion => "Low Cohesion",
            CodeQualityType::HubFile => "Hub File",
            CodeQualityType::DeprecatedDependency => "Deprecated Dependency",
        }
    }
}
