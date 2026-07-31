//! # Graphify Analyze — Graph Metrics & Quality Analysis
//!
//! Analyzes the knowledge graph to find god nodes, architectural patterns,
//! code quality issues, and surprising connections.

use graphify_core::metrics::{NodeMetrics, CodeQualityIssue, CodeQualityType};
use graphify_core::node::{GraphNode, NodeType};
use graphify_core::edge::{GraphEdge, EdgeRelation};
use graphify_core::confidence::Confidence;
use graphify_core::KnowledgeGraph;
use std::collections::{HashMap, HashSet};

/// Compute node metrics for all nodes in the graph.
pub fn compute_metrics(kg: &KnowledgeGraph) -> Vec<NodeMetrics> {
    let mut metrics = Vec::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut out_degree: HashMap<&str, usize> = HashMap::new();

    for edge in &kg.edges {
        *in_degree.entry(edge.target.as_str()).or_default() += 1;
        *out_degree.entry(edge.source.as_str()).or_default() += 1;
    }

    let max_degree = in_degree
        .values()
        .chain(out_degree.values())
        .max()
        .copied()
        .unwrap_or(1)
        .max(1) as f64;

    for node in &kg.nodes {
        let in_d = *in_degree.get(node.id.as_str()).unwrap_or(&0);
        let out_d = *out_degree.get(node.id.as_str()).unwrap_or(&0);
        let total = in_d + out_d;

        metrics.push(NodeMetrics {
            node_id: node.id.clone(),
            in_degree: in_d,
            out_degree: out_d,
            total_degree: total,
            betweenness: 0.0,
            closeness: 0.0,
            pagerank: total as f64 / max_degree,
            eigenvector: 0.0,
            is_god_node: total > 50,
            is_leaf: out_d == 0,
            clustering_coefficient: 0.0,
        });
    }

    metrics
}

/// Identify god nodes — the most highly connected architectural hubs.
pub fn god_nodes(kg: &KnowledgeGraph, top_k: usize) -> Vec<(String, String, usize)> {
    let mut degree_map: HashMap<&str, (String, usize)> = HashMap::new();

    for edge in &kg.edges {
        degree_map
            .entry(edge.source.as_str())
            .or_insert_with(|| (edge.source.clone(), 0))
            .1 += 1;
        degree_map
            .entry(edge.target.as_str())
            .or_insert_with(|| (edge.target.clone(), 0))
            .1 += 1;
    }

    // Replace IDs with labels from the node map
    let node_labels: HashMap<&str, &str> =
        kg.nodes.iter().map(|n| (n.id.as_str(), n.label.as_str())).collect();

    let mut degrees: Vec<(String, String, usize)> = degree_map
        .into_values()
        .map(|(id, deg)| {
            let label = node_labels.get(id.as_str()).unwrap_or(&"unknown");
            (id.clone(), label.to_string(), deg)
        })
        .collect();

    degrees.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
    degrees.truncate(top_k);
    degrees
}

/// Detect code quality issues from graph structure.
pub fn detect_quality_issues(kg: &KnowledgeGraph) -> Vec<CodeQualityIssue> {
    let mut issues = Vec::new();

    // Find god classes (nodes with in_degree > 50 and class type)
    let in_degree: HashMap<&str, usize> = {
        let mut map = HashMap::new();
        for edge in &kg.edges {
            *map.entry(edge.target.as_str()).or_default() += 1;
        }
        map
    };

    // God classes
    for node in &kg.nodes {
        if matches!(node.node_type, NodeType::Class) {
            if let Some(&deg) = in_degree.get(node.id.as_str()) {
                if deg > 50 {
                    issues.push(CodeQualityIssue {
                        issue_type: CodeQualityType::GodClass,
                        nodes: vec![node.id.clone()],
                        severity: (deg as f64 / 100.0).min(1.0),
                        description: format!(
                            "{} has {} incoming dependencies — consider splitting",
                            node.label, deg
                        ),
                        suggestion: Some(
                            "Break this class into smaller, focused classes with single responsibilities"
                                .into(),
                        ),
                    });
                }
            }
        }
    }

    // Circular dependencies
    let mut adjacency: HashMap<&str, HashSet<&str>> = HashMap::new();
    for edge in &kg.edges {
        adjacency
            .entry(edge.source.as_str())
            .or_default()
            .insert(edge.target.as_str());
    }

    let mut found_cycles: HashSet<Vec<String>> = HashSet::new();
    for (start, targets) in &adjacency {
        for target in targets {
            if let Some(back_edges) = adjacency.get(target) {
                if back_edges.contains(start) {
                    let mut cycle = vec![start.to_string(), target.to_string()];
                    cycle.sort();
                    if !found_cycles.contains(&cycle) {
                        found_cycles.insert(cycle.clone());
                        issues.push(CodeQualityIssue {
                            issue_type: CodeQualityType::CircularDependency,
                            nodes: vec![start.to_string(), target.to_string()],
                            severity: 0.6,
                            description: format!(
                                "Circular dependency between {} and {}",
                                start, target
                            ),
                            suggestion: Some(
                                "Introduce an interface or intermediary to break the cycle".into(),
                            ),
                        });
                    }
                }
            }
        }
    }

    // Dead code (leaf nodes of type function with in_degree 0)
    for node in &kg.nodes {
        if node.node_type == NodeType::Function {
            let in_d = *in_degree.get(node.id.as_str()).unwrap_or(&0);
            let out_d = adjacency.get(node.id.as_str()).map(|s| s.len()).unwrap_or(0);
            if in_d == 0 && out_d == 0 {
                issues.push(CodeQualityIssue {
                    issue_type: CodeQualityType::DeadCode,
                    nodes: vec![node.id.clone()],
                    severity: 0.3,
                    description: format!("{} appears to be unused", node.label),
                    suggestion: Some("Consider removing or documenting this function".into()),
                });
            }
        }
    }

    issues
}

/// Find surprising (unexpected) connections between distant communities.
pub fn surprising_connections(
    kg: &KnowledgeGraph,
    communities: &[graphify_core::community::Community],
) -> Vec<(String, String, String)> {
    let mut node_to_community: HashMap<&str, usize> = HashMap::new();
    for comm in communities {
        for node_id in &comm.nodes {
            node_to_community.insert(node_id.as_str(), comm.id);
        }
    }

    let mut surprises = Vec::new();
    for edge in &kg.edges {
        let src_comm = node_to_community.get(edge.source.as_str());
        let tgt_comm = node_to_community.get(edge.target.as_str());

        if let (Some(&sc), Some(&tc)) = (src_comm, tgt_comm) {
            if sc != tc {
                // Cross-community edge — potentially surprising
                if edge.confidence == Confidence::Inferred || edge.confidence == Confidence::Ambiguous {
                    surprises.push((
                        edge.source.clone(),
                        edge.target.clone(),
                        format!("Community {} ↔ Community {}", sc, tc),
                    ));
                }
            }
        }
    }

    surprises
}

/// Analyze the architecture of the project from the graph.
#[derive(Debug, Clone)]
pub struct ArchitectureAnalysis {
    /// Total nodes and edges
    pub total_nodes: usize,
    pub total_edges: usize,
    /// God nodes (architectural hubs)
    pub god_nodes: Vec<(String, String, usize)>,
    /// Code quality issues
    pub issues: Vec<CodeQualityIssue>,
    /// Architecture style detected
    pub architecture_style: Option<String>,
    /// Layers detected (if layered architecture)
    pub layers: Vec<LayerInfo>,
    /// Health score (0.0 - 1.0)
    pub health_score: f64,
}

#[derive(Debug, Clone)]
pub struct LayerInfo {
    pub name: String,
    pub node_count: usize,
    pub dependencies_up: Vec<String>,
    pub dependencies_down: Vec<String>,
}

/// Analyze the overall architecture quality.
pub fn analyze_architecture(kg: &KnowledgeGraph) -> ArchitectureAnalysis {
    let god_nodes = god_nodes(kg, 10);
    let issues = detect_quality_issues(kg);

    // Compute health score
    let health_score = if issues.is_empty() {
        1.0
    } else {
        let avg_severity: f64 = issues.iter().map(|i| i.severity).sum::<f64>() / issues.len() as f64;
        1.0 - avg_severity * 0.5
    };

    // Detect architecture style
    let style = detect_architecture_style(kg, &god_nodes);

    ArchitectureAnalysis {
        total_nodes: kg.nodes.len(),
        total_edges: kg.edges.len(),
        god_nodes,
        issues,
        architecture_style: style,
        layers: Vec::new(),
        health_score,
    }
}

fn detect_architecture_style(
    kg: &KnowledgeGraph,
    _god_nodes: &[(String, String, usize)],
) -> Option<String> {
    // Heuristic: count import vs call edges
    let mut import_count = 0;
    let mut call_count = 0;
    let mut contains_count = 0;

    for edge in &kg.edges {
        match edge.relation {
            EdgeRelation::Imports | EdgeRelation::ImportsFrom => import_count += 1,
            EdgeRelation::Calls => call_count += 1,
            EdgeRelation::Contains => contains_count += 1,
            _ => {}
        }
    }

    if contains_count > call_count && contains_count > import_count {
        Some("Modular Hierarchical".into())
    } else if import_count > call_count {
        Some("Package-Oriented".into())
    } else if call_count > import_count {
        Some("Service-Oriented".into())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphify_core::node::{GraphNode, NodeType};
    use graphify_core::edge::{GraphEdge, EdgeRelation};

    fn make_test_kg() -> KnowledgeGraph {
        KnowledgeGraph {
            schema_version: "2.0".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            project_root: "/test".into(),
            metadata: Default::default(),
            nodes: vec![
                GraphNode::new("hub", "HubClass", NodeType::Class),
                GraphNode::new("leaf1", "leafFunc1", NodeType::Function),
                GraphNode::new("leaf2", "leafFunc2", NodeType::Function),
                GraphNode::new("a", "ClassA", NodeType::Class),
                GraphNode::new("b", "ClassB", NodeType::Class),
            ],
            edges: vec![
                GraphEdge::new("hub", "leaf1", EdgeRelation::Calls),
                GraphEdge::new("hub", "leaf2", EdgeRelation::Calls),
                GraphEdge::new("a", "b", EdgeRelation::Calls),
                GraphEdge::new("b", "a", EdgeRelation::Calls), // circular
            ],
            hyperedges: vec![],
            communities: vec![],
            stats: Default::default(),
        }
    }

    #[test]
    fn test_god_nodes() {
        let kg = make_test_kg();
        let gods = god_nodes(&kg, 3);
        let labels: Vec<&str> = gods.iter().map(|(_, l, _)| l.as_str()).collect();
        assert!(labels.contains(&"HubClass"), "HubClass should be in top god nodes"); // Most connected
    }

    #[test]
    fn test_detect_circular_dependency() {
        let kg = make_test_kg();
        let issues = detect_quality_issues(&kg);
        let circular: Vec<_> = issues
            .iter()
            .filter(|i| i.issue_type == CodeQualityType::CircularDependency)
            .collect();
        assert!(!circular.is_empty());
    }
}
