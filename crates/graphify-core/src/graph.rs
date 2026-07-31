//! Graph operations — building and manipulating the knowledge graph via petgraph.

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Bfs, EdgeRef, IntoNodeReferences};
use petgraph::algo;
use petgraph::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::confidence::Confidence;
use crate::edge::GraphEdge;
use crate::node::GraphNode;
use crate::metrics::NodeMetrics;
use crate::impact::{ImpactAnalysis, ImpactNode, ImpactType};
use crate::{GraphStats, ConfidenceDistribution, KnowledgeGraph};

/// In-memory graph representation backed by petgraph.
pub struct GraphDB {
    /// The directed graph
    graph: DiGraph<GraphNode, GraphEdge>,
    /// Node ID -> NodeIndex mapping for fast lookups
    node_index: HashMap<String, NodeIndex>,
}

impl GraphDB {
    /// Create an empty graph database.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
        }
    }

    /// Build a GraphDB from a KnowledgeGraph.
    pub fn from_knowledge_graph(kg: &KnowledgeGraph) -> Self {
        let mut db = Self::new();

        // Add all nodes
        for node in &kg.nodes {
            db.add_node(node.clone());
        }

        // Add all edges
        for edge in &kg.edges {
            let source_idx = db.node_index.get(&edge.source);
            let target_idx = db.node_index.get(&edge.target);
            if let (Some(&s), Some(&t)) = (source_idx, target_idx) {
                db.graph.add_edge(s, t, edge.clone());
            }
        }

        db
    }

    /// Export to a KnowledgeGraph.
    pub fn to_knowledge_graph(&self, project_root: String) -> KnowledgeGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for idx in self.graph.node_indices() {
            let node = self.graph[idx].clone();
            nodes.push(node);
        }

        for edge_ref in self.graph.edge_references() {
            let mut edge = edge_ref.weight().clone();
            edge.source = self.graph[edge_ref.source()].id.clone();
            edge.target = self.graph[edge_ref.target()].id.clone();
            edges.push(edge);
        }

        let stats = self.compute_stats();

        KnowledgeGraph {
            schema_version: "2.0".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            project_root,
            metadata: Default::default(),
            nodes,
            edges,
            hyperedges: Vec::new(),
            communities: Vec::new(),
            stats,
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: GraphNode) -> NodeIndex {
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        idx
    }

    /// Add an edge between two nodes.
    pub fn add_edge(&mut self, source: &str, target: &str, edge: GraphEdge) -> Option<()> {
        let source_idx = *self.node_index.get(source)?;
        let target_idx = *self.node_index.get(target)?;
        self.graph.add_edge(source_idx, target_idx, edge);
        Some(())
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.node_index.get(id).map(|&idx| &self.graph[idx])
    }

    /// Find nodes by label (fuzzy match).
    pub fn find_nodes(&self, query: &str) -> Vec<(String, &GraphNode)> {
        let lower = query.to_lowercase();
        self.graph
            .node_references()
            .filter(|(_, node)| node.label.to_lowercase().contains(&lower))
            .map(|(_idx, node)| (node.id.clone(), node))
            .collect()
    }

    /// BFS traversal from a starting node.
    pub fn bfs_traverse(&self, start_id: &str, max_depth: usize) -> Vec<(String, usize)> {
        let start_idx = match self.node_index.get(start_id) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        let mut bfs = Bfs::new(&self.graph, start_idx);
        let mut results = Vec::new();
        let mut depth: HashMap<NodeIndex, usize> = HashMap::new();
        depth.insert(start_idx, 0);

        while let Some(node) = bfs.next(&self.graph) {
            let d = *depth.get(&node).unwrap_or(&0);
            if d > max_depth {
                continue;
            }
            results.push((self.graph[node].id.clone(), d));

            for neighbor in self.graph.neighbors(node) {
                depth.entry(neighbor).or_insert(d + 1);
            }
        }

        results
    }

    /// Find shortest path between two nodes.
    pub fn shortest_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        let from_idx = *self.node_index.get(from)?;
        let to_idx = *self.node_index.get(to)?;

        let path = algo::astar(
            &self.graph,
            from_idx,
            |n| n == to_idx,
            |_| 1,
            |_| 0,
        );

        path.map(|(_, nodes)| {
            nodes
                .into_iter()
                .map(|idx| self.graph[idx].id.clone())
                .collect()
        })
    }

    /// Find all paths between two nodes up to a max depth.
    pub fn all_paths(&self, from: &str, to: &str, max_depth: usize) -> Vec<Vec<String>> {
        let from_idx = match self.node_index.get(from) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };
        let to_idx = match self.node_index.get(to) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        let mut paths = Vec::new();
        let mut stack = VecDeque::new();
        stack.push_back((from_idx, vec![from_idx]));

        while let Some((current, path)) = stack.pop_front() {
            if path.len() > max_depth {
                continue;
            }
            if current == to_idx {
                paths.push(
                    path.into_iter()
                        .map(|idx| self.graph[idx].id.clone())
                        .collect(),
                );
                continue;
            }
            for neighbor in self.graph.neighbors_directed(current, Direction::Outgoing) {
                if !path.contains(&neighbor) {
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    stack.push_back((neighbor, new_path));
                }
            }
        }

        paths
    }

    // ── Graph metrics ───────────────────────────────────────────────────────────

    /// Compute statistics for the graph.
    pub fn compute_stats(&self) -> GraphStats {
        let node_count = self.graph.node_count();
        let edge_count = self.graph.edge_count();
        let avg_degree = if node_count > 0 {
            (2.0 * edge_count as f64) / node_count as f64
        } else {
            0.0
        };

        // Density: actual edges / possible edges
        let density = if node_count > 1 {
            (edge_count as f64) / (node_count as f64 * (node_count as f64 - 1.0))
        } else {
            1.0
        };

        // Connected components via Kosaraju
        let sccs = algo::kosaraju_scc(&self.graph);
        let connected_components = sccs.len();
        let is_connected = connected_components <= 1;

        // Confidence distribution
        let mut conf_dist = ConfidenceDistribution::default();
        for edge in self.graph.edge_weights() {
            match edge.confidence {
                Confidence::Extracted => conf_dist.extracted += 1,
                Confidence::Inferred => conf_dist.inferred += 1,
                Confidence::Ambiguous => conf_dist.ambiguous += 1,
            }
        }

        GraphStats {
            node_count,
            edge_count,
            hyperedge_count: 0,
            community_count: 0,
            avg_degree,
            density,
            connected_components,
            is_connected,
            confidence_distribution: conf_dist,
        }
    }

    /// Compute metrics for all nodes.
    pub fn compute_node_metrics(&self) -> Vec<NodeMetrics> {
        let mut metrics = Vec::new();

        // PageRank
        let pagerank = algo::page_rank(&self.graph, 0.85, 100);

        for idx in self.graph.node_indices() {
            let node = &self.graph[idx];
            let in_degree = self
                .graph
                .neighbors_directed(idx, Direction::Incoming)
                .count();
            let out_degree = self
                .graph
                .neighbors_directed(idx, Direction::Outgoing)
                .count();
            let total_degree = in_degree + out_degree;

            metrics.push(NodeMetrics {
                node_id: node.id.clone(),
                in_degree,
                out_degree,
                total_degree,
                betweenness: 0.0, // Computed lazily if needed
                closeness: 0.0,
                pagerank: pagerank[idx.index()],
                eigenvector: 0.0,
                is_god_node: total_degree > 50,
                is_leaf: out_degree == 0,
                clustering_coefficient: 0.0,
            });
        }

        metrics
    }

    /// Identify god nodes (top-K most connected).
    pub fn god_nodes(&self, top_k: usize) -> Vec<(String, String, usize)> {
        let mut degrees: Vec<_> = self
            .graph
            .node_indices()
            .map(|idx| {
                let node = &self.graph[idx];
                let degree = self
                    .graph
                    .neighbors_undirected(idx)
                    .count();
                (node.id.clone(), node.label.clone(), degree)
            })
            .collect();

        degrees.sort_by(|a, b| b.2.cmp(&a.2));
        degrees.truncate(top_k);
        degrees
    }


    // ── Impact analysis ─────────────────────────────────────────────────────────

    /// Analyze the impact of changing a set of nodes.
    pub fn impact_analysis(&self, changed_ids: &[String], max_depth: usize) -> ImpactAnalysis {
        let mut direct = Vec::new();
        let mut indirect = Vec::new();
        let affected_communities = HashSet::new();
        let mut visited: HashMap<String, usize> = HashMap::new();

        // BFS from each changed node
        for changed in changed_ids {
            visited.insert(changed.clone(), 0);
            let mut queue = VecDeque::new();
            queue.push_back((changed.clone(), 0));

            while let Some((node_id, depth)) = queue.pop_front() {
                if depth >= max_depth {
                    continue;
                }

                let node_idx = match self.node_index.get(&node_id) {
                    Some(&idx) => idx,
                    None => continue,
                };

                for edge in self.graph.edges_directed(node_idx, Direction::Outgoing) {
                    let target = &self.graph[edge.target()];
                    let new_depth = depth + 1;

                    if visited.contains_key(&target.id) && visited[&target.id] <= new_depth {
                        continue;
                    }
                    visited.insert(target.id.clone(), new_depth);

                    let impact_node = ImpactNode {
                        node_id: target.id.clone(),
                        label: target.label.clone(),
                        distance: new_depth,
                        impact_type: if new_depth == 1 {
                            ImpactType::MustChange
                        } else if new_depth <= 2 {
                            ImpactType::LikelyAffected
                        } else {
                            ImpactType::ShouldReview
                        },
                        reason: {
                            let distance_label = if new_depth == 1 {
                                "Directly connected".to_string()
                            } else {
                                format!("{} hops away", new_depth)
                            };
                            format!(
                                "{} via {} edge from {}",
                                distance_label,
                                edge.weight().relation.label(),
                                &self.graph[edge.source()].label,
                            )
                        },
                        probability: if new_depth == 1 { 1.0 } else { 0.7 / new_depth as f64 },
                    };

                    if new_depth == 1 {
                        direct.push(impact_node);
                    } else {
                        indirect.push(impact_node);
                    }

                    queue.push_back((target.id.clone(), new_depth));
                }
            }
        }

        // Sort indirect impacts by probability
        indirect.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap_or(std::cmp::Ordering::Equal));

        let blast_radius = direct.len() + indirect.len();
        let risk_score = if blast_radius == 0 {
            0.0
        } else {
            (blast_radius as f64 / self.graph.node_count() as f64).min(1.0)
        };

        ImpactAnalysis {
            changed_nodes: changed_ids.to_vec(),
            direct_impact: direct,
            indirect_impact: indirect,
            blast_radius,
            risk_score,
            affected_communities: affected_communities.into_iter().collect(),
            change_order: Vec::new(),
            estimated_lines_affected: blast_radius * 50, // rough estimate
        }
    }

    // ── Multi-repo query ────────────────────────────────────────────────────────

    /// Merge another graph into this one.
    pub fn merge(&mut self, other: &GraphDB, prefix: &str) -> Result<(), anyhow::Error> {
        let mut id_map: HashMap<String, String> = HashMap::new();

        for idx in other.graph.node_indices() {
            let node = &other.graph[idx];
            let new_id = format!("{}::{}", prefix, node.id);
            id_map.insert(node.id.clone(), new_id.clone());

            let mut new_node = node.clone();
            new_node.id = new_id;
            self.add_node(new_node);
        }

        for edge_ref in other.graph.edge_references() {
            let edge = edge_ref.weight();
            // Read from OTHER graph, not self.graph (critical bug fix)
            let source_id = other.graph[edge_ref.source()].id.clone();
            let target_id = other.graph[edge_ref.target()].id.clone();

            let new_source = id_map.get(&source_id).cloned().unwrap_or(source_id);
            let new_target = id_map.get(&target_id).cloned().unwrap_or(target_id);

            let mut new_edge = edge.clone();
            new_edge.source = new_source.clone();
            new_edge.target = new_target.clone();
            self.add_edge(&new_source, &new_target, new_edge);
        }

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::{GraphEdge, EdgeRelation};
    use crate::node::NodeType;

    fn create_test_node(id: &str, label: &str, ntype: NodeType) -> GraphNode {
        GraphNode::new(id, label, ntype)
    }

    #[test]
    fn test_graph_creation() {
        let mut db = GraphDB::new();
        let n1 = create_test_node("a", "Node A", NodeType::Class);
        let n2 = create_test_node("b", "Node B", NodeType::Function);

        db.add_node(n1);
        db.add_node(n2);

        assert_eq!(db.graph.node_count(), 2);
        assert!(db.get_node("a").is_some());
        assert!(db.get_node("b").is_some());
        assert!(db.get_node("c").is_none());
    }

    #[test]
    fn test_bfs_traversal() {
        let mut db = GraphDB::new();
        let a = db.add_node(create_test_node("a", "A", NodeType::Class));
        let b = db.add_node(create_test_node("b", "B", NodeType::Function));
        let c = db.add_node(create_test_node("c", "C", NodeType::Function));

        db.graph.add_edge(a, b, GraphEdge::new("a", "b", EdgeRelation::Calls));
        db.graph.add_edge(b, c, GraphEdge::new("b", "c", EdgeRelation::Calls));

        let results = db.bfs_traverse("a", 2);
        assert!(results.iter().any(|(id, _)| id == "a"));
        assert!(results.iter().any(|(id, _)| id == "b"));
        assert!(results.iter().any(|(id, _)| id == "c"));
    }

    #[test]
    fn test_shortest_path() {
        let mut db = GraphDB::new();
        let a = db.add_node(create_test_node("a", "A", NodeType::Class));
        let b = db.add_node(create_test_node("b", "B", NodeType::Function));
        let c = db.add_node(create_test_node("c", "C", NodeType::Function));

        db.graph.add_edge(a, b, GraphEdge::new("a", "b", EdgeRelation::Calls));
        db.graph.add_edge(b, c, GraphEdge::new("b", "c", EdgeRelation::Calls));

        let path = db.shortest_path("a", "c").unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_god_nodes() {
        let mut db = GraphDB::new();
        let a = db.add_node(create_test_node("hub", "Hub", NodeType::Class));
        let b = db.add_node(create_test_node("leaf1", "Leaf 1", NodeType::Function));
        let c = db.add_node(create_test_node("leaf2", "Leaf 2", NodeType::Function));
        let d = db.add_node(create_test_node("leaf3", "Leaf 3", NodeType::Function));

        db.graph.add_edge(a, b, GraphEdge::new("hub", "leaf1", EdgeRelation::Calls));
        db.graph.add_edge(a, c, GraphEdge::new("hub", "leaf2", EdgeRelation::Calls));
        db.graph.add_edge(a, d, GraphEdge::new("hub", "leaf3", EdgeRelation::Calls));

        let gods = db.god_nodes(3);
        assert_eq!(gods[0].0, "hub");
    }

    #[test]
    fn test_merge_graphs() {
        let mut db1 = GraphDB::new();
        db1.add_node(create_test_node("a", "A", NodeType::Class));
        db1.add_node(create_test_node("b", "B", NodeType::Function));
        db1.add_edge("a", "b", GraphEdge::new("a", "b", EdgeRelation::Calls));

        let mut db2 = GraphDB::new();
        db2.add_node(create_test_node("x", "X", NodeType::Class));
        db2.add_node(create_test_node("y", "Y", NodeType::Function));
        db2.add_edge("x", "y", GraphEdge::new("x", "y", EdgeRelation::Calls));

        db1.merge(&db2, "repo2").unwrap();

        assert!(db1.get_node("repo2::x").is_some());
        assert!(db1.get_node("repo2::y").is_some());
    }
}
