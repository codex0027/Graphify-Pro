//! Impact analysis types — change prediction and blast radius.

use serde::{Deserialize, Serialize};

/// Impact analysis result for a proposed change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// The node(s) being changed
    pub changed_nodes: Vec<String>,
    /// Directly affected nodes
    pub direct_impact: Vec<ImpactNode>,
    /// Indirectly affected nodes (ripple effect)
    pub indirect_impact: Vec<ImpactNode>,
    /// Estimated blast radius (node count)
    pub blast_radius: usize,
    /// Risk score (0.0 - 1.0)
    pub risk_score: f64,
    /// Communities affected
    pub affected_communities: Vec<usize>,
    /// Suggested order of changes
    pub change_order: Vec<ChangeStep>,
    /// Total estimated lines affected
    pub estimated_lines_affected: usize,
}

/// A node affected by a change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    /// Node ID
    pub node_id: String,
    /// Node label
    pub label: String,
    /// How many hops from the change
    pub distance: usize,
    /// Type of impact
    pub impact_type: ImpactType,
    /// Why this node is affected
    pub reason: String,
    /// Probability of being affected
    pub probability: f64,
}

/// Type of impact on a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactType {
    /// Must be changed (compile error if not)
    MustChange,
    /// Likely needs update
    LikelyAffected,
    /// May need review
    ShouldReview,
    /// Informational — no action needed
    Informational,
}

/// A step in a suggested change order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeStep {
    /// Order number
    pub order: usize,
    /// Node to change
    pub node_id: String,
    /// Description of the change
    pub description: String,
    /// Dependencies that must be changed first
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prerequisites: Vec<String>,
}
