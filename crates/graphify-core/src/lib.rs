//! # Graphify Core — Data Models & Types
//!
//! The foundational data structures for Graphify Pro's knowledge graph engine.
//! Defines nodes, edges, hyperedges, confidence tagging, community structures,
//! and the graph database itself.

pub mod confidence;
pub mod graph;
pub mod node;
pub mod edge;
pub mod hyperedge;
pub mod community;
pub mod query;
pub mod impact;
pub mod metrics;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// The main knowledge graph database structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// Schema version for forward/backward compatibility
    pub schema_version: String,
    /// When the graph was created
    pub created_at: DateTime<Utc>,
    /// When the graph was last updated
    pub updated_at: DateTime<Utc>,
    /// The project root directory
    pub project_root: String,
    /// Project metadata
    pub metadata: GraphMetadata,
    /// All nodes in the graph
    pub nodes: Vec<node::GraphNode>,
    /// All edges (links) connecting nodes
    pub edges: Vec<edge::GraphEdge>,
    /// Hyper-edges connecting 3+ nodes
    pub hyperedges: Vec<hyperedge::HyperEdge>,
    /// Detected communities (subsystems)
    pub communities: Vec<community::Community>,
    /// Graph-level statistics
    pub stats: GraphStats,
}

/// Project-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphMetadata {
    /// Project name from filesystem
    pub project_name: Option<String>,
    /// Primary language detected
    pub primary_language: Option<String>,
    /// All detected languages
    pub languages: Vec<String>,
    /// Total files indexed
    pub total_files: usize,
    /// Lines of code indexed
    pub total_lines: usize,
    /// Git branch (if available)
    pub git_branch: Option<String>,
    /// Git commit hash (if available)
    pub git_commit: Option<String>,
}

/// Graph-level statistics for quick insight.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphStats {
    /// Total node count
    pub node_count: usize,
    /// Total edge count
    pub edge_count: usize,
    /// Total hyperedge count
    pub hyperedge_count: usize,
    /// Number of communities detected
    pub community_count: usize,
    /// Average node degree (in + out)
    pub avg_degree: f64,
    /// Graph density (0.0 - 1.0)
    pub density: f64,
    /// Number of connected components
    pub connected_components: usize,
    /// Whether the graph is fully connected
    pub is_connected: bool,
    /// Distribution of edge confidence levels
    pub confidence_distribution: ConfidenceDistribution,
}

/// Distribution of edge confidence levels in the graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceDistribution {
    pub extracted: usize,
    pub inferred: usize,
    pub ambiguous: usize,
}

/// The output directory structure for graph persistence.
#[derive(Debug, Clone)]
pub struct GraphOutput {
    /// Root output directory (default: graphify-out/)
    pub root: std::path::PathBuf,
    /// Path to graph.json
    pub graph_json: std::path::PathBuf,
    /// Path to GRAPH_REPORT.md
    pub report_md: std::path::PathBuf,
    /// Path to graph.html (interactive visualization)
    pub graph_html: std::path::PathBuf,
    /// Path to manifest.json (file tracking)
    pub manifest_json: std::path::PathBuf,
}

impl GraphOutput {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        let root = root.into();
        Self {
            graph_json: root.join("graph.json"),
            report_md: root.join("GRAPH_REPORT.md"),
            graph_html: root.join("graph.html"),
            manifest_json: root.join("manifest.json"),
            root,
        }
    }

    pub fn default_name() -> &'static str {
        "graphify-out"
    }
}
