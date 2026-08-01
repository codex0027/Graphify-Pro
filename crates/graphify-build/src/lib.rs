//! # Graphify Build — Graph Construction
//!
//! Builds the knowledge graph from extracted nodes and edges, handling
//! deduplication, merging, and connection.

pub mod resolve;

use graphify_core::node::GraphNode;
#[cfg(test)]
use graphify_core::node::NodeType;
use graphify_core::edge::{EdgeRelation, GraphEdge};
use graphify_core::confidence::Confidence;
use graphify_core::{KnowledgeGraph, GraphMetadata, GraphStats, ConfidenceDistribution};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Manifest dependency extracted from package manifests.
#[derive(Debug, Clone)]
pub struct ManifestDep {
    pub name: String,
    pub version: Option<String>,
    pub manifest: String,
}

/// Extract dependencies from package manifest files (Cargo.toml, pyproject.toml, go.mod, etc.)
pub fn extract_manifest_deps(project_root: &Path) -> Vec<ManifestDep> {
    let mut deps = Vec::new();

    // Cargo.toml
    let cargo_path = project_root.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_path) {
            if let Ok(toml_val) = content.parse::<toml::Value>() {
                if let Some(deps_table) = toml_val.get("dependencies").and_then(|d| d.as_table()) {
                    for (name, val) in deps_table {
                        let version = match val {
                            toml::Value::String(s) => Some(s.clone()),
                            toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()).map(String::from),
                            _ => None,
                        };
                        deps.push(ManifestDep { name: name.clone(), version, manifest: "Cargo.toml".into() });
                    }
                }
            }
        }
    }

    // pyproject.toml
    let pyproject_path = project_root.join("pyproject.toml");
    if pyproject_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&pyproject_path) {
            if let Ok(toml_val) = content.parse::<toml::Value>() {
                if let Some(py_deps) = toml_val.get("project").and_then(|p| p.get("dependencies")) {
                    if let Some(arr) = py_deps.as_array() {
                        for dep in arr {
                            if let Some(s) = dep.as_str() {
                                let name = s.split([' ', '=', '>', '<', '~', '^', '!', '[', ';']).next().unwrap_or(s);
                                deps.push(ManifestDep { name: name.to_string(), version: None, manifest: "pyproject.toml".into() });
                            }
                        }
                    }
                }
            }
        }
    }

    // go.mod
    let gomod_path = project_root.join("go.mod");
    if gomod_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&gomod_path) {
            let mut in_require_block = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("require (") {
                    in_require_block = true;
                    continue;
                }
                if in_require_block {
                    if trimmed == ")" {
                        in_require_block = false;
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if !parts.is_empty() {
                        let name = parts[0].to_string();
                        let version = parts.get(1).map(|s| s.to_string());
                        deps.push(ManifestDep { name, version, manifest: "go.mod".into() });
                    }
                } else if let Some(rest) = trimmed.strip_prefix("require ") {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if !parts.is_empty() {
                        let name = parts[0].to_string();
                        let version = parts.get(1).map(|s| s.to_string());
                        deps.push(ManifestDep { name, version, manifest: "go.mod".into() });
                    }
                }
            }
        }
    }

    // package.json
    let pkg_path = project_root.join("package.json");
    if pkg_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_path) {
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                for field in &["dependencies", "devDependencies", "peerDependencies"] {
                    if let Some(deps_table) = json_val.get(field).and_then(|d| d.as_object()) {
                        for (name, version) in deps_table {
                            let ver = version.as_str().map(String::from);
                            deps.push(ManifestDep { name: name.clone(), version: ver, manifest: "package.json".into() });
                        }
                    }
                }
            }
        }
    }

    deps
}

/// Manifest entry for cached file state — includes full extraction result
/// so unchanged files can truly skip re-extraction on rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub hash: String,
    pub language: String,
    /// Cached extraction result (serialized as JSON) for skip-extraction on cache hit.
    pub cached_result: Option<serde_json::Value>,
}

/// Build manifest for incremental caching — stores file hashes + cached
/// extraction results so unchanged files skip extraction entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildManifest {
    pub version: String,
    pub project_root: String,
    pub files: Vec<ManifestEntry>,
}

impl BuildManifest {
    /// Load from disk, or create empty.
    pub fn load(path: &Path) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(Self::new)
        } else {
            Self::new()
        }
    }

    fn new() -> Self {
        Self { version: "2.0".into(), project_root: String::new(), files: Vec::new() }
    }

    /// Check if a file has changed since last build.
    pub fn is_unchanged(&self, path: &str, content: &str) -> bool {
        let hash = Self::hash_content(content);
        self.files.iter().any(|e| e.path == path && e.hash == hash && e.cached_result.is_some())
    }

    /// Retrieve a cached extraction result for a file (must call is_unchanged first).
    pub fn get_cached(&self, path: &str) -> Option<&serde_json::Value> {
        self.files.iter().find(|e| e.path == path).and_then(|e| e.cached_result.as_ref())
    }

    /// Compute SHA-256 hash of file content.
    pub fn hash_content(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Update entry for a file — stores hash + full extraction result for future cache hits.
    pub fn update(&mut self, path: String, content: &str, language: String, cached_result: Option<serde_json::Value>) {
        let hash = Self::hash_content(content);
        // Remove old entry if exists
        self.files.retain(|e| e.path != path);
        self.files.push(ManifestEntry { path, hash, language, cached_result });
    }

    /// Save manifest to disk.
    pub fn save(&self, path: &Path) -> Result<(), anyhow::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Build a knowledge graph from extraction results.
pub fn build_graph(
    extractions: &[graphify_extract::ExtractionResult],
    project_root: &str,
) -> KnowledgeGraph {
    let mut all_nodes: Vec<GraphNode> = Vec::new();
    let mut all_edges: Vec<GraphEdge> = Vec::new();
    let mut seen_node_ids: HashSet<String> = HashSet::new();
    let mut languages: HashSet<String> = HashSet::new();
    let mut total_lines = 0;

    for extraction in extractions {
        languages.insert(extraction.language.clone());

        // Deduplicate nodes
        for node in &extraction.nodes {
            if seen_node_ids.contains(&node.id) {
                continue;
            }
            seen_node_ids.insert(node.id.clone());
            all_nodes.push(node.clone());
        }

        // Deduplicate edges (same source, target, relation)
        let mut seen_edges: HashSet<(String, String, String)> = HashSet::new();
        for edge in &extraction.edges {
            let key = (edge.source.clone(), edge.target.clone(), edge.relation.label().to_string());
            if seen_edges.contains(&key) {
                continue;
            }
            seen_edges.insert(key);
            all_edges.push(edge.clone());
        }

        // Estimate lines
        total_lines += extraction.nodes.len() * 10; // rough estimate
    }

    // Deduplicate nodes by ID (in case two files produce same ID)
    deduplicate_nodes(&mut all_nodes);

    // Compute graph statistics
    let node_count = all_nodes.len();
    let edge_count = all_edges.len();

    let primary_language = if !languages.is_empty() {
        // Count language occurrences
        let mut lang_counts: HashMap<String, usize> = HashMap::new();
        for node in &all_nodes {
            if let Some(ref lang) = node.language {
                *lang_counts.entry(lang.clone()).or_default() += 1;
            }
        }
        lang_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
    } else {
        None
    };

    // Identify god nodes (top 1% most connected)
    let mut degree_map: HashMap<String, usize> = HashMap::new();
    for edge in &all_edges {
        *degree_map.entry(edge.source.clone()).or_default() += 1;
        *degree_map.entry(edge.target.clone()).or_default() += 1;
    }

    let god_threshold = if node_count > 0 {
        let mut degrees: Vec<usize> = degree_map.values().cloned().collect();
        degrees.sort_unstable();
        degrees.get((degrees.len() as f64 * 0.99) as usize).cloned().unwrap_or(usize::MAX)
    } else {
        usize::MAX
    };

    for node in &mut all_nodes {
        if let Some(&degree) = degree_map.get(&node.id) {
            if degree >= god_threshold && degree > 10 {
                node.is_god_node = true;
            }
        }
    }

    // Compute confidence distribution
    let mut conf_dist = ConfidenceDistribution::default();
    for edge in &all_edges {
        match edge.confidence {
            Confidence::Extracted => conf_dist.extracted += 1,
            Confidence::Inferred => conf_dist.inferred += 1,
            Confidence::Ambiguous => conf_dist.ambiguous += 1,
        }
    }

    KnowledgeGraph {
        schema_version: "2.0".into(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        project_root: project_root.to_string(),
        metadata: GraphMetadata {
            project_name: Some(
                std::path::Path::new(project_root)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            primary_language,
            languages: languages.into_iter().collect(),
            total_files: extractions.len(),
            total_lines,
            git_branch: None,
            git_commit: None,
        },
        nodes: all_nodes,
        edges: all_edges,
        hyperedges: Vec::new(),
        communities: Vec::new(),
        stats: GraphStats {
            node_count,
            edge_count,
            hyperedge_count: 0,
            community_count: 0,
            avg_degree: if node_count > 0 { (2.0 * edge_count as f64) / node_count as f64 } else { 0.0 },
            density: if node_count > 1 {
                (edge_count as f64) / (node_count as f64 * (node_count as f64 - 1.0))
            } else {
                1.0
            },
            connected_components: 0,
            is_connected: false,
            confidence_distribution: conf_dist,
        },
    }
}

/// Deduplicate nodes by ID, keeping the one with more metadata.
fn deduplicate_nodes(nodes: &mut Vec<GraphNode>) {
    let mut best: HashMap<String, GraphNode> = HashMap::new();

    for node in nodes.drain(..) {
        if let Some(existing) = best.get(&node.id) {
            // Keep the one with more info
            let existing_score = if existing.source_file.is_some() { 1 } else { 0 }
                + if existing.metadata.is_some() { 1 } else { 0 };
            let new_score = if node.source_file.is_some() { 1 } else { 0 }
                + if node.metadata.is_some() { 1 } else { 0 };

            if new_score > existing_score {
                best.insert(node.id.clone(), node);
            }
        } else {
            best.insert(node.id.clone(), node);
        }
    }

    *nodes = best.into_values().collect();
}

/// Prune dangling edges (edges pointing to non-existent nodes).
pub fn prune_dangling_edges(
    nodes: &[GraphNode],
    edges: &mut Vec<GraphEdge>,
) -> usize {
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let before = edges.len();

    edges.retain(|edge| {
        node_ids.contains(edge.source.as_str()) && node_ids.contains(edge.target.as_str())
    });

    before - edges.len()
}

/// Infer additional edges from existing graph structure.
pub fn infer_edges(nodes: &[GraphNode], edges: &mut Vec<GraphEdge>) -> usize {
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    // Infer transitive imports: if A imports B and B imports C, A might reference C
    let imports: HashMap<&str, HashSet<&str>> = {
        let mut map: HashMap<&str, HashSet<&str>> = HashMap::new();
        for edge in edges.iter() {
            if edge.relation == EdgeRelation::Imports || edge.relation == EdgeRelation::ImportsFrom {
                map.entry(edge.source.as_str())
                    .or_default()
                    .insert(edge.target.as_str());
            }
        }
        map
    };

    // Collect new edges separately to avoid borrow conflicts
    let mut new_edges = Vec::new();

    for edge in edges.iter() {
        if edge.relation == EdgeRelation::Imports || edge.relation == EdgeRelation::ImportsFrom {
            if let Some(transitive) = imports.get(edge.target.as_str()) {
                for &target in transitive {
                    if target != edge.source.as_str() && node_ids.contains(target) {
                        new_edges.push(GraphEdge {
                            source: edge.source.clone(),
                            target: target.to_string(),
                            relation: EdgeRelation::References,
                            context: Some("transitive_import".into()),
                            confidence: Confidence::Inferred,
                            source_file: edge.source_file.clone(),
                            source_location: None,
                            weight: 0.3,
                            metadata: None,
                        });
                    }
                }
            }
        }
    }

    let inferred = new_edges.len();
    edges.extend(new_edges);
    inferred
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_graph() {
        let extractions = vec![
            graphify_extract::ExtractionResult {
                file_path: "src/main.rs".into(),
                nodes: vec![
                    GraphNode::new("src_main", "src/main.rs", NodeType::File),
                    GraphNode::new("main_main", "main", NodeType::Function),
                ],
                edges: vec![
                    GraphEdge::new("src_main", "main_main", EdgeRelation::Contains),
                ],
                language: "Rust".into(),
                errors: vec![],
            },
        ];

        let graph = build_graph(&extractions, "/test/project");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.metadata.total_files, 1);
    }

    // ── Incremental Caching Tests ──────────────────────────────────────────

    #[test]
    fn test_manifest_is_unchanged() {
        let mut manifest = BuildManifest::new();
        manifest.update(
            "src/lib.rs".into(),
            "fn main() {}",
            "Rust".into(),
            Some(serde_json::json!({"nodes": [], "edges": []})),
        );

        // Same content = unchanged AND has cached_result
        assert!(manifest.is_unchanged("src/lib.rs", "fn main() {}"));
        // Different content = changed
        assert!(!manifest.is_unchanged("src/lib.rs", "fn main() { println!(); }"));
        // Unknown file = changed
        assert!(!manifest.is_unchanged("src/other.rs", "fn main() {}"));
    }

    #[test]
    fn test_manifest_get_cached() {
        let mut manifest = BuildManifest::new();
        let cached = serde_json::json!({"nodes": [{"id": "n1"}], "edges": []});
        manifest.update("src/lib.rs".into(), "content", "Rust".into(), Some(cached.clone()));

        let retrieved = manifest.get_cached("src/lib.rs");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &cached);
    }

    #[test]
    fn test_manifest_hash_content() {
        let h1 = BuildManifest::hash_content("hello");
        let h2 = BuildManifest::hash_content("hello");
        let h3 = BuildManifest::hash_content("world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        // SHA-256 hex is 64 chars
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_manifest_update_replaces_old() {
        let mut manifest = BuildManifest::new();
        manifest.update("a.rs".into(), "v1", "Rust".into(), None);
        manifest.update("a.rs".into(), "v2", "Rust".into(), None);
        // Only one entry for a.rs
        assert_eq!(manifest.files.iter().filter(|e| e.path == "a.rs").count(), 1);
        // Hash should be from "v2"
        assert_eq!(
            manifest.files.iter().find(|e| e.path == "a.rs").unwrap().hash,
            BuildManifest::hash_content("v2")
        );
    }

    #[test]
    fn test_manifest_without_cached_result_not_unchanged() {
        let mut manifest = BuildManifest::new();
        // Update without cached_result
        manifest.update("src/lib.rs".into(), "fn main() {}", "Rust".into(), None);
        // Content matches but no cached_result = not considered unchanged
        assert!(!manifest.is_unchanged("src/lib.rs", "fn main() {}"));
    }

    #[test]
    fn test_prune_dangling_edges() {
        let nodes = vec![
            GraphNode::new("a", "A", NodeType::Function),
            GraphNode::new("b", "B", NodeType::Function),
        ];
        let mut edges = vec![
            GraphEdge::new("a", "b", EdgeRelation::Calls),
            GraphEdge::new("a", "c", EdgeRelation::Calls), // c doesn't exist
        ];
        let pruned = prune_dangling_edges(&nodes, &mut edges);
        assert_eq!(pruned, 1);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].source, "a");
        assert_eq!(edges[0].target, "b");
    }

    #[test]
    fn test_manifest_save_load_roundtrip() {
        let tmp = std::env::temp_dir().join("graphify_test_manifest.json");
        let mut manifest = BuildManifest::new();
        manifest.update(
            "test.rs".into(),
            "fn test() {}",
            "Rust".into(),
            Some(serde_json::json!({"nodes": [{"id": "x"}]})),
        );
        manifest.project_root = "/tmp/test".into();
        manifest.save(&tmp).unwrap();

        let loaded = BuildManifest::load(&tmp);
        assert_eq!(loaded.project_root, "/tmp/test");
        assert!(loaded.is_unchanged("test.rs", "fn test() {}"));
        assert!(loaded.get_cached("test.rs").is_some());

        // Cleanup
        let _ = std::fs::remove_file(&tmp);
    }
}
