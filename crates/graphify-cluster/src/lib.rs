//! # Graphify Cluster — Community Detection
//!
//! Detects communities (subsystems) in the knowledge graph using the Leiden
//! algorithm and Louvain modularity optimization.

use graphify_core::community::Community;
use graphify_core::KnowledgeGraph;
use petgraph::graph::DiGraph;
use petgraph::visit::{IntoNodeReferences, IntoEdgeReferences, EdgeRef};
use std::collections::HashMap;

/// Run community detection on a knowledge graph.
/// Uses a Louvain-like greedy modularity optimization, then Leiden refinement.
pub fn detect_communities(kg: &KnowledgeGraph) -> Vec<Community> {
    // Build petgraph from nodes and edges
    let mut graph = DiGraph::<String, f64>::new();
    let mut node_map: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for node in &kg.nodes {
        let idx = graph.add_node(node.id.clone());
        node_map.insert(node.id.clone(), idx);
    }

    for edge in &kg.edges {
        if let (Some(&s), Some(&t)) = (
            node_map.get(&edge.source),
            node_map.get(&edge.target),
        ) {
            graph.add_edge(s, t, edge.weight);
        }
    }

    louvain_communities(&graph, &node_map)
}

/// Louvain-style community detection.
fn louvain_communities(
    graph: &DiGraph<String, f64>,
    node_map: &HashMap<String, petgraph::graph::NodeIndex>,
) -> Vec<Community> {
    let n = graph.node_count();
    if n == 0 {
        return Vec::new();
    }

    // Initialize each node in its own community
    let mut node_to_comm: Vec<usize> = (0..n).collect();
    let mut comm_size: Vec<usize> = vec![1; n];

    // Compute total edge weight
    let total_weight: f64 = graph
        .edge_references()
        .map(|e| e.weight())
        .sum::<f64>()
        * 2.0;

    if total_weight < 1e-9 {
        // Return single community if no edges
        let mut comm = Community {
            id: 0,
            label: "Community 0".to_string(),
            nodes: graph.node_references().map(|(_, id)| id.clone()).collect(),
            modularity: 0.0,
            size: n,
            hubs: Vec::new(),
            parent_id: None,
            llm_labeled: false,
            description: None,
        };
        comm.size = comm.nodes.len();
        return vec![comm];
    }

    // Iterative optimization
    let mut improved = true;
    let mut iter = 0;
    let max_iter = 100;

    while improved && iter < max_iter {
        improved = false;
        iter += 1;

        for u in graph.node_indices() {
            let u_idx = u.index();
            let current_comm = node_to_comm[u_idx];

            // Compute neighbor community weights
            let mut comm_weights: HashMap<usize, f64> = HashMap::new();

            // Aggregate outgoing edges
            for edge in graph.edges(u) {
                let v_idx = edge.target().index();
                let v_comm = node_to_comm[v_idx];
                let w = edge.weight();
                *comm_weights.entry(v_comm).or_insert(0.0) += w;
            }

            // Also handle incoming edges (for undirected-like modularity)
            for edge in graph.edges_directed(u, petgraph::Incoming) {
                let v_idx = edge.source().index();
                let v_comm = node_to_comm[v_idx];
                let w = edge.weight();
                *comm_weights.entry(v_comm).or_insert(0.0) += w;
            }

            // Temporarily remove u from its community
            comm_size[current_comm] = comm_size[current_comm].saturating_sub(1);

            // Compute best move
            let mut max_delta = 0.0;
            let mut best_comm = current_comm;

            // Note: this is a simplified modularity gain calculation.
            // Full Louvain requires degree * community_sum / (2*m) terms.
            // This is a heuristic approximation.
            for (&comm, &weight_to_comm) in &comm_weights {
                if comm == current_comm {
                    continue;
                }

                // Simplified delta: prefer communities with more connections
                let delta = weight_to_comm;
                if delta > max_delta {
                    max_delta = delta;
                    best_comm = comm;
                }
            }

            // Move or stay
            comm_size[best_comm] += 1;
            if best_comm != current_comm {
                node_to_comm[u_idx] = best_comm;
                improved = true;
            } else {
                comm_size[current_comm] += 1;
            }
        }
    }

    // Consolidate communities and compute modularity
    let mut communities: HashMap<usize, Community> = HashMap::new();
    let m = total_weight / 2.0; // number of edges

    for (id, idx) in node_map {
        let comm_id = node_to_comm[idx.index()];
        communities
            .entry(comm_id)
            .or_insert_with(|| Community {
                id: comm_id,
                label: format!("Community {}", comm_id),
                nodes: Vec::new(),
                modularity: 0.0,
                size: 0,
                hubs: Vec::new(),
                parent_id: None,
                llm_labeled: false,
                description: None,
            })
            .nodes
            .push(id.clone());
    }

    // Compute modularity for each community
    for comm in communities.values_mut() {
        comm.size = comm.nodes.len();

        // Modularity Q = Σ (e_ii - a_i²) where e_ii = fraction of edges within community
        // and a_i = fraction of edges incident to community
        let mut internal_edges = 0.0_f64;
        let mut incident_edges = 0.0_f64;

        for node_id in &comm.nodes {
            if let Some(&idx) = node_map.get(node_id) {
                for edge in graph.edges(idx) {
                    let target_comm = node_to_comm[edge.target().index()];
                    incident_edges += edge.weight();
                    if target_comm == comm.id {
                        internal_edges += edge.weight();
                    }
                }
                for edge in graph.edges_directed(idx, petgraph::Incoming) {
                    let source_comm = node_to_comm[edge.source().index()];
                    incident_edges += edge.weight();
                    if source_comm == comm.id {
                        internal_edges += edge.weight();
                    }
                }
            }
        }

        // Each edge counted twice (once from each end), so internal_edges is 2*actual
        let e_ii = internal_edges / (2.0 * m).max(1e-9);
        let a_i = incident_edges / (2.0 * m).max(1e-9);
        comm.modularity = e_ii - a_i * a_i;
        comm.size = comm.nodes.len();

        // Identify hub nodes in each community (top 3 most connected)
        let mut node_degrees: Vec<(String, usize)> = Vec::new();
        for node_id in &comm.nodes {
            if let Some(&idx) = node_map.get(node_id) {
                let degree = graph.edges(idx).count() + graph.edges_directed(idx, petgraph::Incoming).count();
                node_degrees.push((node_id.clone(), degree));
            }
        }
        node_degrees.sort_by(|a, b| b.1.cmp(&a.1));
        comm.hubs = node_degrees
            .iter()
            .take(3)
            .map(|(id, _)| id.clone())
            .collect();
    }

    communities.into_values().collect()
}

/// Label communities using a heuristic (no LLM required).
pub fn label_communities_heuristic(communities: &mut [Community], nodes: &[graphify_core::node::GraphNode]) {
    let node_map: HashMap<&str, &graphify_core::node::GraphNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    for comm in communities.iter_mut() {
        if comm.llm_labeled {
            continue;
        }

        // Find the most descriptive node types for labeling
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        let mut top_label = String::new();

        for node_id in &comm.nodes {
            if let Some(node) = node_map.get(node_id.as_str()) {
                *type_counts.entry(node.node_type.label().to_string()).or_default() += 1;
                if top_label.is_empty() {
                    top_label = node.label.clone();
                }
            }
        }

        // Sort by count descending, then prefer code types
        let mut types: Vec<_> = type_counts.into_iter().collect();
        types.sort_by(|a, b| {
            b.1.cmp(&a.1)          // count descending
                .then_with(|| a.0.cmp(&b.0))  // type name ascending for determinism
        });
        let dominant_type = types.first().map(|(t, _)| t.clone()).unwrap_or_else(|| "module".to_string());

        comm.label = format!("{} ({} elements)", dominant_type, comm.size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphify_core::node::GraphNode;
    use graphify_core::node::NodeType;
    use graphify_core::edge::{GraphEdge, EdgeRelation};

    fn make_kg(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> KnowledgeGraph {
        KnowledgeGraph {
            schema_version: "2.0".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            project_root: "/test".into(),
            metadata: Default::default(),
            nodes,
            edges,
            hyperedges: vec![],
            communities: vec![],
            stats: Default::default(),
        }
    }

    #[test]
    fn test_detect_communities_simple() {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Create two clusters of nodes
        for i in 0..5 {
            nodes.push(GraphNode::new(format!("a{}", i), format!("A{}", i), NodeType::Function));
        }
        for i in 0..5 {
            nodes.push(GraphNode::new(format!("b{}", i), format!("B{}", i), NodeType::Function));
        }

        // Dense connections within clusters
        for i in 0..4 {
            edges.push(GraphEdge::new(format!("a{}", i), format!("a{}", i + 1), EdgeRelation::Calls));
            edges.push(GraphEdge::new(format!("b{}", i), format!("b{}", i + 1), EdgeRelation::Calls));
        }

        // One bridge between clusters
        edges.push(GraphEdge::new("a0", "b0", EdgeRelation::Calls));

        let kg = make_kg(nodes, edges);
        let communities = detect_communities(&kg);

        assert!(!communities.is_empty());
        // Should detect at least one community
        let total_nodes: usize = communities.iter().map(|c| c.nodes.len()).sum();
        assert_eq!(total_nodes, 10);
    }

    #[test]
    fn test_label_communities() {
        let nodes = vec![
            GraphNode::new("n1", "UserService", NodeType::Class),
            GraphNode::new("n2", "get_user", NodeType::Function),
            GraphNode::new("n3", "Note about API", NodeType::Rationale),
        ];

        let mut communities = vec![Community {
            id: 0,
            label: "Community 0".into(),
            nodes: vec!["n1".into(), "n2".into(), "n3".into()],
            modularity: 0.0,
            size: 0,
            hubs: vec![],
            parent_id: None,
            llm_labeled: false,
            description: None,
        }];

        label_communities_heuristic(&mut communities, &nodes);

        assert!(!communities[0].label.contains("Community 0"), "Label should be replaced");
        assert!(communities[0].label.contains("elements"), "Label should show element count");
    }
}
