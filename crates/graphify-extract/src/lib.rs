//! # Graphify Extract — Code Extraction via Tree-Sitter + Regex
//!
//! Extracts structural information from source code files using tree-sitter
//! for deterministic AST parsing, with regex fallback for unsupported languages.

pub mod tree_sitter;

use graphify_core::node::{GraphNode, NodeType};
use graphify_core::edge::{EdgeRelation, GraphEdge};
use graphify_core::confidence::Confidence;
use std::collections::HashSet;
use std::path::Path;

/// Result of extracting a single file.
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Path of the extracted file
    pub file_path: String,
    /// Extracted nodes
    pub nodes: Vec<GraphNode>,
    /// Extracted edges
    pub edges: Vec<GraphEdge>,
    /// Detected language
    pub language: String,
    /// Any errors encountered
    pub errors: Vec<String>,
}

/// Configuration for the extraction pipeline.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    /// Maximum number of parallel workers
    pub max_workers: usize,
    /// Whether to extract rationale comments
    pub extract_rationale: bool,
    /// Whether to include code-only extraction (no LLM needed)
    pub code_only: bool,
    /// Root directory for relative paths
    pub root: std::path::PathBuf,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            max_workers: num_cpus::get(),
            extract_rationale: true,
            code_only: true,
            root: std::path::PathBuf::from("."),
        }
    }
}

/// Regex-based code extractor (works without tree-sitter grammars).
/// Extracts basic structural information from source code using regex patterns.
pub struct RegexExtractor;

impl RegexExtractor {
    /// Create a simple file node ID from a relative path.
    pub fn file_node_id(rel_path: &Path) -> String {
        let stem = rel_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let parent = rel_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if parent.is_empty() {
            sanitize_id(stem)
        } else {
            sanitize_id(&format!("{}_{}", parent, stem))
        }
    }

    /// Extract from a Python file.
    pub fn extract_python(content: &str, file_path: &str, config: &ExtractConfig) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut errors = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        // File node
        nodes.push(GraphNode {
            id: file_id.clone(),
            label: file_path.to_string(),
            node_type: NodeType::File,
            source_file: Some(file_path.to_string()),
            source_location: None,
            confidence: Confidence::Extracted,
            is_god_node: false,
            community_id: None,
            metadata: None,
            language: Some("Python".to_string()),
        });

        let re_class = regex::Regex::new(r"class\s+(\w+)(?:\(([^)]*)\))?\s*:").unwrap();
        let re_func = regex::Regex::new(r"(?:async\s+)?def\s+(\w+)\s*\((.*?)\)\s*(?:->\s*(\S+))?\s*:").unwrap();
        let re_import = regex::Regex::new(r"^(?:from\s+(\S+)\s+)?import\s+(.+)$").unwrap();
        let re_call = regex::Regex::new(r"(\w+)\.(\w+)\s*\(").unwrap();
        let re_rationale = regex::Regex::new(r"#\s*(?:NOTE|IMPORTANT|HACK|WHY|RATIONALE|TODO|FIXME):\s*(.+)").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            let loc = format!("L{}", line_num + 1);

            // Class definitions
            for cap in re_class.captures_iter(line) {
                let class_name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, class_name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: class_name.clone(),
                    node_type: NodeType::Class,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Python".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("class_definition".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });

                // Inheritance
                if let Some(bases) = cap.get(2) {
                    for base in bases.as_str().split(',') {
                        let base = base.trim();
                        if !base.is_empty() && base != "object" {
                            let base_id = sanitize_id(base);
                            edges.push(GraphEdge {
                                source: format!("{}_{}", stem, class_name),
                                target: format!("{}_{}", stem, base_id),
                                relation: EdgeRelation::Inherits,
                                context: Some("class_inheritance".into()),
                                confidence: Confidence::Extracted,
                                source_file: Some(file_path.to_string()),
                                source_location: Some(loc.clone()),
                                weight: 1.0,
                                metadata: None,
                            });
                        }
                    }
                }
            }

            // Function definitions
            for cap in re_func.captures_iter(line) {
                let func_name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, func_name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: func_name.clone(),
                    node_type: NodeType::Function,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Python".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("function_definition".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            // Imports
            for cap in re_import.captures_iter(line) {
                if let Some(module) = cap.get(1) {
                    let module_name = module.as_str().to_string();
                    edges.push(GraphEdge {
                        source: file_id.clone(),
                        target: sanitize_id(&module_name),
                        relation: EdgeRelation::ImportsFrom,
                        context: Some("import".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                } else if let Some(names) = cap.get(2) {
                    for name in names.as_str().split(',') {
                        let name = name.trim().split(" as ").next().unwrap_or("").trim();
                        if !name.is_empty() && name != "*" {
                            edges.push(GraphEdge {
                                source: file_id.clone(),
                                target: sanitize_id(name),
                                relation: EdgeRelation::Imports,
                                context: Some("import".into()),
                                confidence: Confidence::Extracted,
                                source_file: Some(file_path.to_string()),
                                source_location: Some(loc.clone()),
                                weight: 1.0,
                                metadata: None,
                            });
                        }
                    }
                }
            }

            // Call expressions (infer function relationships)
            for cap in re_call.captures_iter(line) {
                let obj = cap[1].to_string();
                let method = cap[2].to_string();
                edges.push(GraphEdge {
                    source: format!("{}_{}", stem, obj),
                    target: format!("{}_{}", stem, method),
                    relation: EdgeRelation::Calls,
                    context: Some("method_call".into()),
                    confidence: Confidence::Inferred,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 0.7,
                    metadata: None,
                });
            }

            // Rationale comments
            if config.extract_rationale {
                for cap in re_rationale.captures_iter(line) {
                    let text = cap[1].to_string();
                    let rid = format!("{}_{}_rationale_{}", stem, sanitize_id(&text.chars().take(30).collect::<String>()), line_num + 1);
                    nodes.push(GraphNode {
                        id: rid.clone(),
                        label: text.chars().take(80).collect(),
                        node_type: NodeType::Rationale,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        confidence: Confidence::Extracted,
                        is_god_node: false,
                        community_id: None,
                        metadata: None,
                        language: Some("Python".to_string()),
                    });
                    edges.push(GraphEdge {
                        source: rid,
                        target: file_id.clone(),
                        relation: EdgeRelation::RationaleFor,
                        context: Some("rationale".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                }
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(),
            nodes,
            edges,
            language: "Python".to_string(),
            errors,
        }
    }

    /// Extract from a Rust file.
    pub fn extract_rust(content: &str, file_path: &str, config: &ExtractConfig) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut errors = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        nodes.push(GraphNode {
            id: file_id.clone(),
            label: file_path.to_string(),
            node_type: NodeType::File,
            source_file: Some(file_path.to_string()),
            source_location: None,
            confidence: Confidence::Extracted,
            is_god_node: false,
            community_id: None,
            metadata: None,
            language: Some("Rust".to_string()),
        });

        let re_struct = regex::Regex::new(r"pub\s+(?:struct|enum|trait|impl)\s+(\w+)").unwrap();
        let re_fn = regex::Regex::new(r"pub\s+(?:async\s+)?fn\s+(\w+)\s*[<(]").unwrap();
        let re_mod = regex::Regex::new(r"^(?:pub\s+)?mod\s+(\w+)").unwrap();
        let re_use = regex::Regex::new(r"use\s+(.+?);").unwrap();
        let re_impl_for = regex::Regex::new(r"impl\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap();
        let re_rationale = regex::Regex::new(r"//\s*(?:NOTE|IMPORTANT|HACK|WHY|RATIONALE|TODO|FIXME):\s*(.+)").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            let loc = format!("L{}", line_num + 1);

            for cap in re_struct.captures_iter(line) {
                let name = cap[1].to_string();
                let node_type = if line.trim().starts_with("pub enum") {
                    NodeType::Enum
                } else if line.trim().starts_with("pub trait") {
                    NodeType::Interface
                } else {
                    NodeType::Class
                };
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Rust".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("struct_definition".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_fn.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Function,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Rust".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("function_definition".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_mod.captures_iter(line) {
                let name = cap[1].to_string();
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: sanitize_id(&name),
                    relation: EdgeRelation::Contains,
                    context: Some("module".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_use.captures_iter(line) {
                let path = cap[1].to_string();
                let last = path.split("::").last().unwrap_or(&path);
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: sanitize_id(last),
                    relation: EdgeRelation::Imports,
                    context: Some("use".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_impl_for.captures_iter(line) {
                if let Some(trait_name) = cap.get(1) {
                    let type_name = &cap[2];
                    edges.push(GraphEdge {
                        source: format!("{}_{}", stem, type_name),
                        target: sanitize_id(trait_name.as_str()),
                        relation: EdgeRelation::Implements,
                        context: Some("impl_trait".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                }
            }

            if config.extract_rationale {
                for cap in re_rationale.captures_iter(line) {
                    let text = cap[1].to_string();
                    let rid = format!("{}_rationale_{}", stem, line_num + 1);
                    nodes.push(GraphNode {
                        id: rid.clone(),
                        label: text.chars().take(80).collect(),
                        node_type: NodeType::Rationale,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        confidence: Confidence::Extracted,
                        is_god_node: false,
                        community_id: None,
                        metadata: None,
                        language: Some("Rust".to_string()),
                    });
                    edges.push(GraphEdge {
                        source: rid,
                        target: file_id.clone(),
                        relation: EdgeRelation::RationaleFor,
                        context: Some("rationale".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                }
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(),
            nodes,
            edges,
            language: "Rust".to_string(),
            errors,
        }
    }

    /// Extract from a JavaScript/TypeScript file.
    pub fn extract_js(content: &str, file_path: &str, is_typescript: bool) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let errors = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
        let lang = if is_typescript { "TypeScript" } else { "JavaScript" };

        nodes.push(GraphNode {
            id: file_id.clone(),
            label: file_path.to_string(),
            node_type: NodeType::File,
            source_file: Some(file_path.to_string()),
            source_location: None,
            confidence: Confidence::Extracted,
            is_god_node: false,
            community_id: None,
            metadata: None,
            language: Some(lang.to_string()),
        });

        let re_class = regex::Regex::new(r"(?:export\s+)?(?:abstract\s+)?class\s+(\w+)").unwrap();
        let re_func = regex::Regex::new(r"(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\(").unwrap();
        let re_arrow = regex::Regex::new(r"(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\([^)]*\)\s*=>").unwrap();
        let re_method = regex::Regex::new(r"(?:async\s+)?(\w+)\s*\([^)]*\)\s*\{").unwrap();
        let re_import = regex::Regex::new(r#"import\s+(?:\{[^}]*\}|(\w+))\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        let re_export = regex::Regex::new(r#"export\s+\{[^}]*\}\s+from\s+['"]([^'"]+)['"]"#).unwrap();
        let re_interface = regex::Regex::new(r"(?:export\s+)?interface\s+(\w+)").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            let loc = format!("L{}", line_num + 1);

            for cap in re_class.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Class,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some(lang.to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("class".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_func.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Function,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some(lang.to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("function".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_arrow.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Function,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some(lang.to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("arrow_function".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_import.captures_iter(line) {
                if let Some(module) = cap.get(2) {
                    let module_name = module.as_str();
                    edges.push(GraphEdge {
                        source: file_id.clone(),
                        target: sanitize_id(module_name),
                        relation: EdgeRelation::ImportsFrom,
                        context: Some("import".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                }
            }

            for cap in re_export.captures_iter(line) {
                if let Some(module) = cap.get(1) {
                    edges.push(GraphEdge {
                        source: file_id.clone(),
                        target: sanitize_id(module.as_str()),
                        relation: EdgeRelation::ReExports,
                        context: Some("re-export".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                }
            }

            if is_typescript {
                for cap in re_interface.captures_iter(line) {
                    let name = cap[1].to_string();
                    let node_id = format!("{}_{}", stem, name);
                    nodes.push(GraphNode {
                        id: node_id.clone(),
                        label: name.clone(),
                        node_type: NodeType::Interface,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        confidence: Confidence::Extracted,
                        is_god_node: false,
                        community_id: None,
                        metadata: None,
                        language: Some(lang.to_string()),
                    });
                    edges.push(GraphEdge {
                        source: file_id.clone(),
                        target: node_id,
                        relation: EdgeRelation::Contains,
                        context: Some("interface".into()),
                        confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()),
                        source_location: Some(loc.clone()),
                        weight: 1.0,
                        metadata: None,
                    });
                }
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(),
            nodes,
            edges,
            language: lang.to_string(),
            errors,
        }
    }

    /// Extract from a Go file.
    pub fn extract_go(content: &str, file_path: &str) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut errors = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        nodes.push(GraphNode {
            id: file_id.clone(),
            label: file_path.to_string(),
            node_type: NodeType::File,
            source_file: Some(file_path.to_string()),
            source_location: None,
            confidence: Confidence::Extracted,
            is_god_node: false,
            community_id: None,
            metadata: None,
            language: Some("Go".to_string()),
        });

        let re_struct = regex::Regex::new(r"type\s+(\w+)\s+struct\s*\{").unwrap();
        let re_interface = regex::Regex::new(r"type\s+(\w+)\s+interface\s*\{").unwrap();
        let re_func = regex::Regex::new(r"func\s+(?:\([^)]*\)\s+)?(\w+)\s*\(").unwrap();
        let re_import = regex::Regex::new(r#"import\s+(?:(\w+)\s+)?\"([^\"]+)\""#).unwrap();

        for (line_num, line) in content.lines().enumerate() {
            let loc = format!("L{}", line_num + 1);

            for cap in re_struct.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Class,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Go".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("struct".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_interface.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Interface,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Go".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("interface".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_func.captures_iter(line) {
                let name = cap[1].to_string();
                if name == "init" {
                    continue; // skip init functions
                }
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: name.clone(),
                    node_type: NodeType::Function,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some("Go".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: node_id,
                    relation: EdgeRelation::Contains,
                    context: Some("function".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }

            for cap in re_import.captures_iter(line) {
                let path = cap[2].to_string();
                let last = path.rsplit('/').next().unwrap_or(&path);
                edges.push(GraphEdge {
                    source: file_id.clone(),
                    target: sanitize_id(last),
                    relation: EdgeRelation::Imports,
                    context: Some("import".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(loc.clone()),
                    weight: 1.0,
                    metadata: None,
                });
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(),
            nodes,
            edges,
            language: "Go".to_string(),
            errors,
        }
    }

    /// Generic regex extraction for languages without dedicated tree-sitter grammar.
    pub fn extract_generic(
        content: &str, file_path: &str, language: &str, config: &ExtractConfig,
    ) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        nodes.push(GraphNode {
            id: file_id.clone(), label: file_path.to_string(), node_type: NodeType::File,
            source_file: Some(file_path.to_string()), source_location: None,
            confidence: Confidence::Extracted, is_god_node: false, community_id: None,
            metadata: None, language: Some(language.to_string()),
        });

        // Common patterns across most languages
        let re_class = regex::Regex::new(r"(?:class|struct|object|data\s+class)\s+(\w+)").unwrap();
        let re_func = regex::Regex::new(r"(?:fun|fn|func|function|def|sub)\s+(\w+)\s*\(").unwrap();
        let re_import = regex::Regex::new(r"(?:import|require|using|#include|open)\s+(.+?)(?:;|$)").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            let loc = format!("L{}", line_num + 1);

            for cap in re_class.captures_iter(line) {
                let name = cap[1].to_string();
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(), label: name.clone(), node_type: NodeType::Class,
                    source_file: Some(file_path.to_string()), source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted, is_god_node: false, community_id: None,
                    metadata: None, language: Some(language.to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(), target: node_id, relation: EdgeRelation::Contains,
                    context: Some("class".into()), confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()), source_location: Some(loc.clone()),
                    weight: 1.0, metadata: None,
                });
            }

            for cap in re_func.captures_iter(line) {
                let name = cap[1].to_string();
                if name.is_empty() || name == "main" { continue; }
                let node_id = format!("{}_{}", stem, name);
                nodes.push(GraphNode {
                    id: node_id.clone(), label: name.clone(), node_type: NodeType::Function,
                    source_file: Some(file_path.to_string()), source_location: Some(loc.clone()),
                    confidence: Confidence::Extracted, is_god_node: false, community_id: None,
                    metadata: None, language: Some(language.to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(), target: node_id, relation: EdgeRelation::Contains,
                    context: Some("function".into()), confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()), source_location: Some(loc.clone()),
                    weight: 1.0, metadata: None,
                });
            }

            for cap in re_import.captures_iter(line) {
                let import = cap[1].trim().trim_end_matches(';').trim();
                if !import.is_empty() && import != "*" {
                    let last = import.split(['.', '/', ':', '\\']).last().unwrap_or(import);
                    edges.push(GraphEdge {
                        source: file_id.clone(), target: sanitize_id(last), relation: EdgeRelation::Imports,
                        context: Some("import".into()), confidence: Confidence::Extracted,
                        source_file: Some(file_path.to_string()), source_location: Some(loc.clone()),
                        weight: 1.0, metadata: None,
                    });
                }
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(), nodes, edges,
            language: language.to_string(), errors: Vec::new(),
        }
    }

    /// Extract structure from TOML files.
    pub fn extract_toml(content: &str, file_path: &str) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);

        nodes.push(GraphNode {
            id: file_id.clone(), label: file_path.to_string(), node_type: NodeType::File,
            source_file: Some(file_path.to_string()), source_location: None,
            confidence: Confidence::Extracted, is_god_node: false, community_id: None,
            metadata: None, language: Some("TOML".to_string()),
        });

        let re_section = regex::Regex::new(r"^\[(.+?)\]").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = re_section.captures(line) {
                let name = cap[1].to_string().trim_matches('"').to_string();
                let node_id = sanitize_id(&name.replace('.', "_"));
                nodes.push(GraphNode {
                    id: node_id.clone(), label: name.clone(), node_type: NodeType::Class,
                    source_file: Some(file_path.to_string()), source_location: Some(format!("L{}", line_num + 1)),
                    confidence: Confidence::Extracted, is_god_node: false, community_id: None,
                    metadata: None, language: Some("TOML".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(), target: node_id, relation: EdgeRelation::Contains,
                    context: Some("section".into()), confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(format!("L{}", line_num + 1)),
                    weight: 1.0, metadata: None,
                });
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(), nodes, edges,
            language: "TOML".to_string(), errors: Vec::new(),
        }
    }

    /// Extract structure from Markdown files.
    pub fn extract_markdown(content: &str, file_path: &str) -> ExtractionResult {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let rel_path = Path::new(file_path);
        let file_id = Self::file_node_id(rel_path);

        nodes.push(GraphNode {
            id: file_id.clone(), label: file_path.to_string(), node_type: NodeType::File,
            source_file: Some(file_path.to_string()), source_location: None,
            confidence: Confidence::Extracted, is_god_node: false, community_id: None,
            metadata: None, language: Some("Markdown".to_string()),
        });

        let re_heading = regex::Regex::new(r"^(#{1,6})\s+(.+)").unwrap();
        let re_link = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        let re_wikilink = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = re_heading.captures(line) {
                let level = cap[1].len();
                let title = cap[2].to_string();
                let node_id = sanitize_id(&title.replace(' ', "_"));
                let node_type = if level == 1 { NodeType::Class } else { NodeType::Function };
                nodes.push(GraphNode {
                    id: node_id.clone(), label: title.clone(), node_type,
                    source_file: Some(file_path.to_string()), source_location: Some(format!("L{}", line_num + 1)),
                    confidence: Confidence::Extracted, is_god_node: false, community_id: None,
                    metadata: Some(serde_json::json!({"level": level})),
                    language: Some("Markdown".to_string()),
                });
                edges.push(GraphEdge {
                    source: file_id.clone(), target: node_id, relation: EdgeRelation::Contains,
                    context: Some("heading".into()), confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(format!("L{}", line_num + 1)),
                    weight: 1.0, metadata: None,
                });
            }

            for cap in re_link.captures_iter(line) {
                let text = cap[1].to_string();
                let url = cap[2].to_string();
                edges.push(GraphEdge {
                    source: file_id.clone(), target: sanitize_id(&text),
                    relation: EdgeRelation::References, context: Some(url),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(format!("L{}", line_num + 1)),
                    weight: 0.5, metadata: None,
                });
            }

            for cap in re_wikilink.captures_iter(line) {
                let target = cap[1].to_string();
                edges.push(GraphEdge {
                    source: file_id.clone(), target: sanitize_id(&target),
                    relation: EdgeRelation::References, context: Some("wikilink".into()),
                    confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()),
                    source_location: Some(format!("L{}", line_num + 1)),
                    weight: 0.7, metadata: None,
                });
            }
        }

        ExtractionResult {
            file_path: file_path.to_string(), nodes, edges,
            language: "Markdown".to_string(), errors: Vec::new(),
        }
    }

    /// Dispatch extraction based on file extension. Tries tree-sitter first, falls back to regex.
    pub fn extract_file(
        content: &str,
        file_path: &str,
        config: &ExtractConfig,
    ) -> ExtractionResult {
        let path = Path::new(file_path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Try tree-sitter first
        if let Some(ts_lang) = tree_sitter::TsLanguage::from_extension(&ext) {
            return tree_sitter::TreeSitterExtractor::extract(content, file_path, ts_lang, config);
        }

        // Fall back to regex for unsupported languages
        match ext.as_str() {
            "py" | "pyi" => Self::extract_python(content, file_path, config),
            "rs" => Self::extract_rust(content, file_path, config),
            "js" | "jsx" | "cjs" | "mjs" => Self::extract_js(content, file_path, false),
            "ts" | "tsx" | "mts" | "cts" => Self::extract_js(content, file_path, true),
            "go" => Self::extract_go(content, file_path),
            // Generic extraction for 15 additional languages via regex
            "kt" | "kts" => Self::extract_generic(content, file_path, "Kotlin", config),
            "lua" => Self::extract_generic(content, file_path, "Lua", config),
            "dart" => Self::extract_generic(content, file_path, "Dart", config),
            "sql" => Self::extract_generic(content, file_path, "SQL", config),
            "r" | "R" => Self::extract_generic(content, file_path, "R", config),
            "erl" => Self::extract_generic(content, file_path, "Erlang", config),
            "toml" => Self::extract_toml(content, file_path),
            "vue" => Self::extract_generic(content, file_path, "Vue", config),
            "md" | "mdx" => Self::extract_markdown(content, file_path),
            // Additional regex fallbacks for parity with original Graphify
            "cls" | "trigger" => Self::extract_generic(content, file_path, "Apex", config),
            "blade" => Self::extract_generic(content, file_path, "Blade", config),
            "cshtml" | "razor" => Self::extract_generic(content, file_path, "Razor", config),
            "pas" | "pp" => Self::extract_generic(content, file_path, "Pascal", config),
            "dm" | "dme" => Self::extract_generic(content, file_path, "DreamMaker", config),
            "groovy" | "gvy" | "gy" | "gsh" => Self::extract_generic(content, file_path, "Groovy", config),
            "svelte" => Self::extract_generic(content, file_path, "Svelte", config),
            "astro" => Self::extract_generic(content, file_path, "Astro", config),
            "ps1" | "psm1" | "psd1" => Self::extract_generic(content, file_path, "PowerShell", config),
            "f" | "f90" | "f95" | "f03" | "f08" => Self::extract_generic(content, file_path, "Fortran", config),
            "m" | "mm" => Self::extract_generic(content, file_path, "Objective-C", config),
            _ => ExtractionResult {
                file_path: file_path.to_string(),
                nodes: vec![GraphNode {
                    id: Self::file_node_id(path),
                    label: file_path.to_string(),
                    node_type: NodeType::File,
                    source_file: Some(file_path.to_string()),
                    source_location: None,
                    confidence: Confidence::Extracted,
                    is_god_node: false,
                    community_id: None,
                    metadata: None,
                    language: Some(format!("unknown ({})", ext)),
                }],
                edges: Vec::new(),
                language: format!("unknown ({})", ext),
                errors: vec![format!("No extractor for .{} files yet", ext)],
            },
        }
    }
}

/// Sanitize a string for use as a node ID.
pub fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_python() {
        let content = r#"
import os
from pathlib import Path

class UserService:
    def get_user(self, id: int) -> User:
        # NOTE: This uses the legacy API
        return db.query(f"SELECT * FROM users WHERE id={id}")

def main():
    svc = UserService()
    svc.get_user(1)
"#;
        let config = ExtractConfig::default();
        let result = RegexExtractor::extract_python(content, "app/service.py", &config);
        assert!(!result.nodes.is_empty());
        assert!(!result.edges.is_empty());
        assert!(result.nodes.iter().any(|n| n.label.contains("UserService")));
    }

    #[test]
    fn test_extract_rust() {
        let content = r#"
use std::collections::HashMap;

pub struct UserService {
    db: Database,
}

impl UserService {
    pub fn get_user(&self, id: u64) -> User {
        // NOTE: Legacy API compatibility
        self.db.query_user(id)
    }
}
"#;
        let config = ExtractConfig::default();
        let result = RegexExtractor::extract_rust(content, "src/service.rs", &config);
        assert!(result.nodes.iter().any(|n| n.label.contains("UserService")));
    }
}
