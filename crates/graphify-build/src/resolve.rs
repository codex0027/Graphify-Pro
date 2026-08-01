//! # Cross-File Symbol Resolution
//!
//! Resolves cross-file import references by building a symbol table and
//! matching import edges to actual nodes in other files. This turns
//! `A imports X` + `X is defined in B` into `A references X (in B)`.
//!
//! All resolved edges are tagged `Confidence::Inferred`.

use graphify_core::confidence::Confidence;
use graphify_core::edge::{EdgeRelation, GraphEdge};
use graphify_core::node::GraphNode;
use std::collections::{HashMap, HashSet};

/// Resolve cross-file import references.
///
/// Builds a symbol table from all nodes, then for each import edge,
/// looks up the imported name in the symbol table. If the symbol
/// exists in a different file, creates a `References` edge.
///
/// Returns the number of resolved edges added.
pub fn resolve_cross_file_references(
    nodes: &[GraphNode],
    edges: &mut Vec<GraphEdge>,
) -> usize {
    // Build symbol table: label → set of file IDs that define it
    let mut symbol_table: HashMap<String, HashSet<String>> = HashMap::new();
    for node in nodes {
        // Only index code-level symbols (classes, functions, interfaces, enums)
        if node.node_type.is_code() {
            let key = normalize_symbol(&node.label);
            symbol_table
                .entry(key)
                .or_default()
                .insert(node.id.clone());
            // Also index the simple name (last segment)
            if let Some(simple) = node.label.rsplit('.').next() {
                symbol_table
                    .entry(normalize_symbol(simple))
                    .or_default()
                    .insert(node.id.clone());
            }
            if let Some(simple) = node.label.rsplit("::").next() {
                symbol_table
                    .entry(normalize_symbol(simple))
                    .or_default()
                    .insert(node.id.clone());
            }
        }
    }

    // Build file → node mapping for source file resolution
    let mut file_nodes: HashMap<String, String> = HashMap::new();
    for node in nodes {
        if let Some(ref file) = node.source_file {
            file_nodes.entry(file.clone()).or_insert(node.id.clone());
        }
    }

    // Collect new resolved edges
    let mut new_edges = Vec::new();
    let mut resolved = 0usize;
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    for edge in edges.iter() {
        if edge.relation != EdgeRelation::Imports
            && edge.relation != EdgeRelation::ImportsFrom
        {
            continue;
        }

        let target_label = edge.target.as_str();
        let normalized = normalize_symbol(target_label);

        // Try to find the imported symbol in the symbol table
        if let Some(matches) = symbol_table.get(&normalized) {
            for matched_id in matches {
                // Only resolve if the match is in a DIFFERENT file than the source node
                let source_node_file = nodes
                    .iter()
                    .find(|n| n.id == edge.source)
                    .and_then(|n| n.source_file.as_deref());
                let matched_node_file = nodes
                    .iter()
                    .find(|n| n.id == *matched_id)
                    .and_then(|n| n.source_file.as_deref());
                let same_file = source_node_file.is_some()
                    && matched_node_file.is_some()
                    && source_node_file == matched_node_file;

                if !same_file && node_ids.contains(matched_id.as_str()) {
                    new_edges.push(GraphEdge {
                        source: edge.source.clone(),
                        target: matched_id.clone(),
                        relation: EdgeRelation::References,
                        context: Some("cross_file_resolved".into()),
                        confidence: Confidence::Inferred,
                        source_file: edge.source_file.clone(),
                        source_location: edge.source_location.clone(),
                        weight: 0.6,
                        metadata: None,
                    });
                    resolved += 1;
                }
            }
        }

        // Also try to match import target to file nodes
        if let Some(file_node_id) = file_nodes.get(&normalized) {
            if file_node_id != &edge.source {
                new_edges.push(GraphEdge {
                    source: edge.source.clone(),
                    target: file_node_id.clone(),
                    relation: EdgeRelation::References,
                    context: Some("cross_file_file_resolved".into()),
                    confidence: Confidence::Inferred,
                    source_file: edge.source_file.clone(),
                    source_location: edge.source_location.clone(),
                    weight: 0.5,
                    metadata: None,
                });
                resolved += 1;
            }
        }
    }

    // Deduplicate new edges before adding (collect into owned set to avoid borrow conflict)
    let existing_keys: HashSet<(String, String)> = edges
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();

    for new_edge in new_edges {
        if !existing_keys.contains(&(new_edge.source.clone(), new_edge.target.clone())) {
            edges.push(new_edge);
        }
    }

    resolved
}

/// Normalize a symbol name for lookup (lowercase, strip common prefixes/suffixes).
fn normalize_symbol(s: &str) -> String {
    s.trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ';')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphify_core::node::NodeType;

    #[test]
    fn test_resolve_cross_file_python_import() {
        let nodes = vec![
            // File A: auth.py — defines authenticate()
            GraphNode {
                id: "auth_authenticate".into(),
                label: "authenticate".into(),
                node_type: NodeType::Function,
                source_file: Some("auth.py".into()),
                ..GraphNode::new("auth_authenticate", "authenticate", NodeType::Function)
            },
            // File B: main.py — imports authenticate
            GraphNode {
                id: "main_main".into(),
                label: "main.py".into(),
                node_type: NodeType::File,
                source_file: Some("main.py".into()),
                ..GraphNode::new("main_main", "main.py", NodeType::File)
            },
        ];

        let mut edges = vec![
            // main.py imports authenticate
            GraphEdge::new("main_main", "authenticate", EdgeRelation::ImportsFrom),
        ];

        let resolved = resolve_cross_file_references(&nodes, &mut edges);

        // Should have resolved the import to the actual authenticate() function
        assert!(resolved > 0, "Should resolve at least one cross-file reference");
        assert!(
            edges.iter().any(|e| {
                e.relation == EdgeRelation::References
                    && e.source == "main_main"
                    && e.target == "auth_authenticate"
                    && e.confidence == Confidence::Inferred
            }),
            "Should have inferred reference from main to auth_authenticate"
        );
    }

    #[test]
    fn test_no_cross_file_when_same_file() {
        let nodes = vec![
            GraphNode {
                id: "lib_user".into(),
                label: "User".into(),
                node_type: NodeType::Class,
                source_file: Some("src/lib.rs".into()),
                ..GraphNode::new("lib_user", "User", NodeType::Class)
            },
            GraphNode {
                id: "lib_main".into(),
                label: "src/lib.rs".into(),
                node_type: NodeType::File,
                source_file: Some("src/lib.rs".into()),
                ..GraphNode::new("lib_main", "src/lib.rs", NodeType::File)
            },
        ];

        let mut edges = vec![
            GraphEdge::new("lib_main", "user", EdgeRelation::Imports),
        ];

        let resolved = resolve_cross_file_references(&nodes, &mut edges);

        // User is in the same file (src/lib.rs) — should NOT resolve cross-file
        assert_eq!(resolved, 0, "Should not resolve same-file symbols");
    }
}
