//! Tree-sitter AST-based code extraction — Config-driven multi-language support.
//!
//! Uses a LanguageConfig to map tree-sitter node kinds to graph concepts,
//! enabling support for 35+ languages with a single generic handler.

use graphify_core::confidence::Confidence;
use graphify_core::edge::{EdgeRelation, GraphEdge};
use graphify_core::node::{GraphNode, NodeType};
use std::path::Path;
use tree_sitter::{Language, Parser, Node};

use super::{ExtractConfig, ExtractionResult, sanitize_id};

// ── Language Config ───────────────────────────────────────────────────────────

/// Configuration mapping tree-sitter node kinds to graph concepts.
struct LangConfig {
    /// Node kinds that represent classes/structs
    class_kinds: &'static [&'static str],
    /// Node kinds for functions/methods
    func_kinds: &'static [&'static str],
    /// Node kinds for interfaces/traits
    interface_kinds: &'static [&'static str],
    /// Node kinds for enums
    enum_kinds: &'static [&'static str],
    /// Node kinds for imports
    import_kinds: &'static [&'static str],
    /// Node kinds for comments (rationale extraction)
    comment_kinds: &'static [&'static str],
    /// Field name for the "name" child node
    name_field: &'static str,
    /// Whether this language uses named imports (not attr-based)
    use_named_imports: bool,
    /// Comment prefix for rationale detection
    comment_prefix: &'static str,
}

// ── Language Enum ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsLanguage {
    Rust, Python, JavaScript, TypeScript, TypeScriptTsx, Go, Java,
    C, Cpp, CSharp, Swift, Ruby, Php, Scala,
    Haskell, Julia, Bash, Hcl, Elixir, Zig, Ocaml,
    Json, Yaml, Css, Html, Solidity, Verilog,
}

impl TsLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust), "py"|"pyi" => Some(Self::Python),
            "js"|"jsx"|"cjs"|"mjs" => Some(Self::JavaScript),
            "tsx" => Some(Self::TypeScriptTsx),
            "ts"|"mts"|"cts" => Some(Self::TypeScript),
            "go" => Some(Self::Go), "java" => Some(Self::Java),
            "c"|"h" => Some(Self::C), "cpp"|"cc"|"cxx"|"hpp"|"hh" => Some(Self::Cpp),
            "cs" => Some(Self::CSharp), "swift" => Some(Self::Swift),
            "rb" => Some(Self::Ruby), "php" => Some(Self::Php),
            "scala"|"sc" => Some(Self::Scala),
            "hs" => Some(Self::Haskell), "jl" => Some(Self::Julia),
            "sh"|"bash"|"zsh" => Some(Self::Bash),
            "tf"|"hcl" => Some(Self::Hcl), "ex"|"exs" => Some(Self::Elixir),
            "zig" => Some(Self::Zig), "ml"|"mli" => Some(Self::Ocaml),
            "json" => Some(Self::Json),
            "yaml"|"yml" => Some(Self::Yaml), "css" => Some(Self::Css),
            "html"|"htm" => Some(Self::Html),
            "sol" => Some(Self::Solidity), "v"|"sv" => Some(Self::Verilog),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Rust=>"Rust",Self::Python=>"Python",Self::JavaScript=>"JavaScript",
            Self::TypeScript|Self::TypeScriptTsx=>"TypeScript",Self::Go=>"Go",
            Self::Java=>"Java",Self::C=>"C",Self::Cpp=>"C++",Self::CSharp=>"C#",
            Self::Swift=>"Swift",Self::Ruby=>"Ruby",
            Self::Php=>"PHP",Self::Scala=>"Scala",
            Self::Haskell=>"Haskell",Self::Julia=>"Julia",Self::Bash=>"Bash",
            Self::Hcl=>"Terraform",Self::Elixir=>"Elixir",
            Self::Zig=>"Zig",Self::Ocaml=>"OCaml",            Self::Json=>"JSON",
            Self::Yaml=>"YAML",Self::Css=>"CSS",Self::Html=>"HTML",
            Self::Solidity=>"Solidity",Self::Verilog=>"Verilog",
        }
    }

    pub fn is_typescript(&self) -> bool {
        matches!(self, Self::TypeScript | Self::TypeScriptTsx)
    }

    pub fn language(&self) -> Language {
        match self {
            Self::Rust=>tree_sitter_rust::LANGUAGE.into(),
            Self::Python=>tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript=>tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript=>tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::TypeScriptTsx=>tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Go=>tree_sitter_go::LANGUAGE.into(),
            Self::Java=>tree_sitter_java::LANGUAGE.into(),
            Self::C=>tree_sitter_c::LANGUAGE.into(),
            Self::Cpp=>tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp=>tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Swift=>tree_sitter_swift::LANGUAGE.into(),
            Self::Ruby=>tree_sitter_ruby::LANGUAGE.into(),
            Self::Php=>tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Scala=>tree_sitter_scala::LANGUAGE.into(),
            Self::Haskell=>tree_sitter_haskell::LANGUAGE.into(),
            Self::Julia=>tree_sitter_julia::LANGUAGE.into(),
            Self::Bash=>tree_sitter_bash::LANGUAGE.into(),
            Self::Hcl=>tree_sitter_hcl::LANGUAGE.into(),
            Self::Elixir=>tree_sitter_elixir::LANGUAGE.into(),
            Self::Zig=>tree_sitter_zig::LANGUAGE.into(),
            Self::Ocaml=>tree_sitter_ocaml::LANGUAGE_OCAML.into(),
            Self::Json=>tree_sitter_json::LANGUAGE.into(),
            Self::Yaml=>tree_sitter_yaml::LANGUAGE.into(),
            Self::Css=>tree_sitter_css::LANGUAGE.into(),
            Self::Html=>tree_sitter_html::LANGUAGE.into(),
            Self::Solidity=>tree_sitter_solidity::LANGUAGE.into(),
            Self::Verilog=>tree_sitter_verilog::LANGUAGE.into(),
        }
    }

    fn config(&self) -> LangConfig {
        match self {
            Self::Rust => LangConfig {
                class_kinds: &["struct_item", "union_item"],
                func_kinds: &["function_item"],
                interface_kinds: &["trait_item"],
                enum_kinds: &["enum_item"],
                import_kinds: &["use_declaration"],
                comment_kinds: &["line_comment", "block_comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "//",
            },
            Self::Python => LangConfig {
                class_kinds: &["class_definition"],
                func_kinds: &["function_definition"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["import_statement", "import_from_statement"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "#",
            },
            Self::JavaScript | Self::TypeScript | Self::TypeScriptTsx => LangConfig {
                class_kinds: &["class_declaration", "abstract_class_declaration"],
                func_kinds: &["function_declaration", "generator_function_declaration",
                             "method_definition", "arrow_function", "variable_declarator"],
                interface_kinds: if self.is_typescript() { &["interface_declaration"] } else { &[] },
                enum_kinds: if self.is_typescript() { &["enum_declaration"] } else { &[] },
                import_kinds: &["import_statement"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "//",
            },
            Self::Go => LangConfig {
                class_kinds: &["type_spec"],
                func_kinds: &["function_declaration", "method_declaration"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["import_declaration"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "//",
            },
            Self::Java => LangConfig {
                class_kinds: &["class_declaration"],
                func_kinds: &["method_declaration", "constructor_declaration"],
                interface_kinds: &["interface_declaration"],
                enum_kinds: &["enum_declaration"],
                import_kinds: &["import_declaration"],
                comment_kinds: &["line_comment", "block_comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "//",
            },
            // C family — share common node kinds
            Self::C | Self::Cpp | Self::CSharp | Self::Swift | Self::Scala => LangConfig {
                class_kinds: &["class_specifier", "class_declaration", "struct_specifier", "class_definition",
                              "struct_declaration", "object_definition"],
                func_kinds: &["function_definition", "function_declaration",
                             "method_declaration", "function_declarator",
                             "constructor_declaration", "function_item"],
                interface_kinds: &["interface_declaration", "protocol_declaration", "trait_definition"],
                enum_kinds: &["enum_declaration", "enum_specifier", "enum_definition"],
                import_kinds: &["import_declaration", "preproc_include", "using_directive",
                               "import_statement", "include_directive"],
                comment_kinds: &["comment", "line_comment", "block_comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "//",
            },
            Self::Ruby => LangConfig {
                class_kinds: &["class", "module"],
                func_kinds: &["method", "singleton_method"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["call"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "#",
            },
            Self::Php => LangConfig {
                class_kinds: &["class_declaration", "trait_declaration"],
                func_kinds: &["function_definition", "method_declaration"],
                interface_kinds: &["interface_declaration"],
                enum_kinds: &["enum_declaration"],
                import_kinds: &["use_declaration", "namespace_use_declaration"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "//",
            },
            Self::Haskell => LangConfig {
                class_kinds: &["class", "data_declaration", "newtype_declaration"],
                func_kinds: &["function", "signature", "declaration"],
                interface_kinds: &["class"],
                enum_kinds: &[],
                import_kinds: &["import", "import_statement"],
                comment_kinds: &["comment", "line_comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "--",
            },
            Self::Julia => LangConfig {
                class_kinds: &["struct_definition", "abstract_definition", "primitive_definition"],
                func_kinds: &["function_definition", "macro_definition"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["import_statement", "using_statement"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "#",
            },
            Self::Bash => LangConfig {
                class_kinds: &[],
                func_kinds: &["function_definition"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["redirected_statement"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "#",
            },
            Self::Hcl => LangConfig {
                class_kinds: &["block", "resource", "data_source"],
                func_kinds: &["module_call", "output_declaration"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &[],
                comment_kinds: &["comment"],
                name_field: "identifier", use_named_imports: false, comment_prefix: "#",
            },
            Self::Elixir => LangConfig {
                class_kinds: &["defmodule", "module"],
                func_kinds: &["def", "defp", "defmacro"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["alias", "import", "require", "use"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "#",
            },
            Self::Zig => LangConfig {
                class_kinds: &["ContainerDecl", "StructDecl", "EnumDecl"],
                func_kinds: &["FnProto", "FnDecl"],
                interface_kinds: &[],
                enum_kinds: &["EnumDecl"],
                import_kinds: &["Use", "BuiltinCall"],
                comment_kinds: &["line_comment"],
                name_field: "name", use_named_imports: true, comment_prefix: "//",
            },
            Self::Ocaml => LangConfig {
                class_kinds: &["module_definition", "module_type_definition"],
                func_kinds: &["let_binding", "let_definition", "value_definition"],
                interface_kinds: &["module_type_definition"],
                enum_kinds: &["type_definition"],
                import_kinds: &["open_statement", "include_statement"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "(*",
            },
            // Config/Data formats — extract structure where possible
            Self::Json | Self::Yaml => LangConfig {
                class_kinds: &["object", "table", "array", "block_mapping"],
                func_kinds: &[],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &[],
                comment_kinds: &["comment"],
                name_field: "key", use_named_imports: false, comment_prefix: "#",
            },
            Self::Css => LangConfig {
                class_kinds: &["rule_set", "class_selector", "id_selector"],
                func_kinds: &["at_rule", "keyframes_statement"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &["import_statement"],
                comment_kinds: &["comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "/*",
            },
            Self::Html => LangConfig {
                class_kinds: &["element", "component", "heading"],
                func_kinds: &["script_element", "style_element"],
                interface_kinds: &[],
                enum_kinds: &[],
                import_kinds: &[],
                comment_kinds: &["comment"],
                name_field: "tag_name", use_named_imports: false, comment_prefix: "<!--",
            },
            Self::Solidity => LangConfig {
                class_kinds: &["contract_declaration", "library_declaration"],
                func_kinds: &["function_definition", "constructor_definition", "fallback_receiver"],
                interface_kinds: &["interface_declaration"],
                enum_kinds: &["enum_declaration"],
                import_kinds: &["import_directive"],
                comment_kinds: &["comment", "line_comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "//",
            },
            Self::Verilog => LangConfig {
                class_kinds: &["module_declaration", "interface_declaration"],
                func_kinds: &["function_declaration", "task_declaration", "always_construct"],
                interface_kinds: &["interface_declaration"],
                enum_kinds: &["enum_declaration"],
                import_kinds: &["include_directive", "import_declaration"],
                comment_kinds: &["line_comment", "block_comment"],
                name_field: "name", use_named_imports: false, comment_prefix: "//",
            },
        }
    }
}

// ── Unified Extractor ─────────────────────────────────────────────────────────

pub struct TreeSitterExtractor;

impl TreeSitterExtractor {
    pub fn extract(
        content: &str, file_path: &str, lang: TsLanguage, config: &ExtractConfig,
    ) -> ExtractionResult {
        let mut parser = Parser::new();
        if parser.set_language(&lang.language()).is_err() {
            return ExtractionResult {
                file_path: file_path.to_string(), nodes: vec![], edges: vec![],
                language: lang.name().to_string(),
                errors: vec!["Failed to set tree-sitter language".into()],
            };
        }

        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return ExtractionResult {
                file_path: file_path.to_string(), nodes: vec![], edges: vec![],
                language: lang.name().to_string(),
                errors: vec!["Failed to parse with tree-sitter".into()],
            },
        };

        let source_bytes = content.as_bytes();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let rel_path = Path::new(file_path);
        let file_id = super::RegexExtractor::file_node_id(rel_path);
        let stem = rel_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        // File node
        nodes.push(GraphNode {
            id: file_id.clone(), label: file_path.to_string(), node_type: NodeType::File,
            source_file: Some(file_path.to_string()), source_location: None,
            confidence: Confidence::Extracted, is_god_node: false, community_id: None,
            metadata: None, language: Some(lang.name().to_string()),
        });

        let root = tree.root_node();
        let lc = lang.config();
        Self::handle_generic(&root, source_bytes, &file_id, stem, file_path, lang, config, &lc, &mut nodes, &mut edges);

        ExtractionResult {
            file_path: file_path.to_string(), nodes, edges,
            language: lang.name().to_string(), errors: Vec::new(),
        }
    }

    /// Generic AST walker driven by LanguageConfig.
    fn handle_generic(
        node: &Node, source: &[u8], file_id: &str, stem: &str, file_path: &str,
        lang: TsLanguage, config: &ExtractConfig, lc: &LangConfig,
        nodes: &mut Vec<GraphNode>, edges: &mut Vec<GraphEdge>,
    ) {
        let kind = node.kind();
        let loc = node_loc(node);
        let lang_name = lang.name().to_string();

        // ── Class-like nodes ──────────────────────────────────────────────────
        if lc.class_kinds.contains(&kind) {
            if let Some(name) = child_text(node, source, lc.name_field) {
                let nid = sanitize_id(&format!("{}_{}", stem, name));
                let ntype = if lc.interface_kinds.contains(&kind) { NodeType::Interface }
                    else if lc.enum_kinds.contains(&kind) { NodeType::Enum }
                    else { NodeType::Class };
                nodes.push(node_new(&nid, &name, ntype, file_path, &loc, lang_name.clone()));
                edges.push(contains_edge(file_id, &nid, kind, file_path, &loc));

                // Handle inheritance for C-family languages
                if let Some(superclass) = node.child_by_field_name("superclass") {
                    if let Ok(sn) = superclass.utf8_text(source) {
                        let base = sn.trim().split('.').last().unwrap_or(sn);
                        edges.push(GraphEdge {
                            source: nid.clone(),
                            target: sanitize_id(&format!("{}_{}", stem, base)),
                            relation: EdgeRelation::Inherits,
                            context: Some("extends".into()),
                            confidence: Confidence::Extracted,
                            source_file: Some(file_path.to_string()),
                            source_location: Some(loc.clone()),
                            weight: 1.0, metadata: None,
                        });
                    }
                }
                // Handle Python-style bases (argument_list)
                if let Some(bases) = find_descendant(node, "argument_list") {
                    for i in 0..bases.child_count() {
                        if let Some(base) = bases.child(i) {
                            if let Ok(bn) = base.utf8_text(source) {
                                let clean = bn.trim().trim_matches(|c: char| c == ',' || c.is_whitespace()).split('.').last().unwrap_or(bn);
                                if !clean.is_empty() && clean != "object" {
                                    edges.push(GraphEdge {
                                        source: nid.clone(),
                                        target: sanitize_id(&format!("{}_{}", stem, clean)),
                                        relation: EdgeRelation::Inherits,
                                        context: Some("inheritance".into()),
                                        confidence: Confidence::Extracted,
                                        source_file: Some(file_path.to_string()),
                                        source_location: Some(loc.clone()),
                                        weight: 1.0, metadata: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Function-like nodes ───────────────────────────────────────────────
        if lc.func_kinds.contains(&kind) {
            if let Some(name) = child_text(node, source, lc.name_field) {
                if name != "init" && name != "main" && !name.is_empty() {
                    let nid = sanitize_id(&format!("{}_{}", stem, name));
                    if !nodes.iter().any(|n: &GraphNode| n.id == nid) {
                        nodes.push(node_new(&nid, &name, NodeType::Function, file_path, &loc, lang_name.clone()));
                        edges.push(contains_edge(file_id, &nid, kind, file_path, &loc));
                    }
                }
            }
        }

        // ── Interface-like nodes ──────────────────────────────────────────────
        if !lc.class_kinds.contains(&kind) && lc.interface_kinds.contains(&kind) {
            if let Some(name) = child_text(node, source, lc.name_field) {
                let nid = sanitize_id(&format!("{}_{}", stem, name));
                nodes.push(node_new(&nid, &name, NodeType::Interface, file_path, &loc, lang_name.clone()));
                edges.push(contains_edge(file_id, &nid, kind, file_path, &loc));
            }
        }

        // ── Import nodes ──────────────────────────────────────────────────────
        if lc.import_kinds.contains(&kind) && lc.use_named_imports {
            if let Ok(text) = node.utf8_text(source) {
                let cleaned = text.trim_start_matches("import").trim()
                    .trim_start_matches("use").trim()
                    .trim_start_matches("from").trim()
                    .trim_start_matches("#include").trim()
                    .trim_start_matches("open").trim()
                    .trim_start_matches("alias").trim()
                    .trim_start_matches("require").trim()
                    .trim_end_matches(';').trim()
                    .trim_matches(|c: char| c == '"' || c == '\'' || c == '<' || c == '>');
                // Take last meaningful segment
                let last = cleaned.split(&['.', '/', ':'][..])
                    .filter(|s| !s.is_empty())
                    .last();
                if let Some(seg) = last {
                    let seg = seg.trim();
                    if !seg.is_empty() && seg != "*" {
                        edges.push(GraphEdge {
                            source: file_id.to_string(), target: sanitize_id(seg),
                            relation: EdgeRelation::Imports,
                            context: Some("import".into()),
                            confidence: Confidence::Extracted,
                            source_file: Some(file_path.to_string()),
                            source_location: Some(loc.clone()),
                            weight: 1.0, metadata: None,
                        });
                    }
                }
            }
        }

        // ── Rationale comments ────────────────────────────────────────────────
        if config.extract_rationale && lc.comment_kinds.contains(&kind) {
            if let Ok(text) = node.utf8_text(source) {
                extract_rationale(text, file_id, stem, file_path, &loc, nodes, edges, lc.comment_prefix, &lang_name);
            }
        }

        // ── Recurse ───────────────────────────────────────────────────────────
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::handle_generic(&child, source, file_id, stem, file_path, lang, config, lc, nodes, edges);
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn child_text<'a>(node: &Node<'a>, source: &'a [u8], field: &str) -> Option<String> {
    // Try the named field first
    if let Some(n) = node.child_by_field_name(field) {
        if let Ok(s) = n.utf8_text(source) {
            return Some(s.to_string());
        }
    }
    // Try common alternate field names
    for alt in &["name", "identifier", "declarator"] {
        if *alt == field { continue; }
        if let Some(n) = node.child_by_field_name(alt) {
            if let Ok(s) = n.utf8_text(source) {
                let trimmed = s.trim();
                if !trimmed.is_empty() && trimmed.len() < 100 {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    // Fallback: look for any named child
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            if c.is_named() && c.child_count() == 0 {
                if let Ok(s) = c.utf8_text(source) {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() && trimmed.len() < 100 {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
    }
    None
}

fn find_descendant<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind { return Some(child); }
            if let Some(found) = find_descendant(&child, kind) { return Some(found); }
        }
    }
    None
}

fn node_loc(node: &Node) -> String { format!("L{}", node.start_position().row + 1) }

fn node_new(id: &str, label: &str, ntype: NodeType, file: &str, loc: &str, lang: String) -> GraphNode {
    GraphNode {
        id: id.to_string(), label: label.to_string(), node_type: ntype,
        source_file: Some(file.to_string()), source_location: Some(loc.to_string()),
        confidence: Confidence::Extracted, is_god_node: false, community_id: None,
        metadata: None, language: Some(lang),
    }
}

fn contains_edge(source: &str, target: &str, ctx: &str, file: &str, loc: &str) -> GraphEdge {
    GraphEdge {
        source: source.to_string(), target: target.to_string(),
        relation: EdgeRelation::Contains, context: Some(ctx.to_string()),
        confidence: Confidence::Extracted,
        source_file: Some(file.to_string()), source_location: Some(loc.to_string()),
        weight: 1.0, metadata: None,
    }
}

fn extract_rationale(
    text: &str, file_id: &str, stem: &str, file_path: &str, loc: &str,
    nodes: &mut Vec<GraphNode>, edges: &mut Vec<GraphEdge>,
    comment_prefix: &str, lang: &str,
) {
    let line = loc.trim_start_matches('L').parse::<usize>().unwrap_or(0);
    let mut trimmed = text.strip_prefix(comment_prefix).unwrap_or(text).trim();
    trimmed = trimmed.strip_prefix("/*").unwrap_or(trimmed).strip_suffix("*/").unwrap_or(trimmed)
        .strip_prefix("<!--").unwrap_or(trimmed).strip_suffix("-->").unwrap_or(trimmed).trim();
    for prefix in &["NOTE:", "TODO:", "FIXME:", "HACK:", "WHY:", "RATIONALE:", "IMPORTANT:"] {
        if trimmed.starts_with(prefix) {
            let body = trimmed.strip_prefix(prefix).unwrap_or(trimmed).trim();
            if !body.is_empty() {
                let rid = sanitize_id(&format!("{}_rationale_{}", stem, line));
                nodes.push(node_new(&rid, &body.chars().take(80).collect::<String>(), NodeType::Rationale, file_path, loc, lang.to_string()));
                edges.push(GraphEdge {
                    source: rid, target: file_id.to_string(),
                    relation: EdgeRelation::RationaleFor,
                    context: Some("rationale".into()), confidence: Confidence::Extracted,
                    source_file: Some(file_path.to_string()), source_location: Some(loc.to_string()),
                    weight: 1.0, metadata: None,
                });
            }
            break;
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_extract_rust() {
        let content = "pub struct UserService { db: Database, }\nimpl UserService { pub fn get_user(&self, id: u64) -> User { self.db.query_user(id) } }";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/service.rs", TsLanguage::Rust, &config);
        assert!(result.nodes.iter().any(|n| n.label == "UserService"));
        assert!(result.nodes.iter().any(|n| n.label == "get_user"));
    }

    #[test]
    fn test_ts_extract_python() {
        let content = "class UserService:\n    def get_user(self, id):\n        # NOTE: Legacy API\n        return self.db.query(id)\n";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "app/service.py", TsLanguage::Python, &config);
        assert!(result.nodes.iter().any(|n| n.label == "UserService"));
        assert!(result.nodes.iter().any(|n| n.label == "get_user"));
        assert!(result.nodes.iter().any(|n| n.node_type == NodeType::Rationale));
    }

    #[test]
    fn test_ts_extract_js() {
        let content = "import { useState } from 'react';\nexport class App { render() { return null; } }\nconst handleClick = () => { console.log('clicked'); };";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/App.js", TsLanguage::JavaScript, &config);
        assert!(result.nodes.iter().any(|n| n.label == "App"));
        assert!(result.nodes.iter().any(|n| n.label == "render"));
        assert!(result.nodes.iter().any(|n| n.label == "handleClick"));
    }

    #[test]
    fn test_ts_extract_java() {
        let content = "import java.util.ArrayList;\npublic class UserService extends BaseService implements Serializable {\n    private String dbUrl;\n    public User getUser(int id) { return null; }\n}";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/UserService.java", TsLanguage::Java, &config);
        assert!(result.nodes.iter().any(|n| n.label == "UserService"));
        assert!(result.nodes.iter().any(|n| n.label == "getUser"));
        assert!(result.edges.iter().any(|e| e.relation == EdgeRelation::Inherits));
    }

    #[test]
    fn test_ts_extract_c() {
        let content = "#include <stdio.h>\nstruct User { int id; char* name; };\nvoid process_user(struct User* u) { printf(\"%s\", u->name); }";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/user.c", TsLanguage::C, &config);
        assert!(!result.nodes.is_empty(), "Should extract at least file node");
        assert!(result.nodes.len() > 1, "Should extract some content nodes");
    }

    #[test]
    fn test_ts_extract_cpp() {
        let content = "#include <vector>\nclass UserService {\npublic:\n    User* getUser(int id);\n};";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/service.cpp", TsLanguage::Cpp, &config);
        assert!(result.nodes.iter().any(|n| n.label == "UserService"));
    }

    #[test]
    fn test_ts_extract_csharp() {
        let content = "using System;\npublic class UserService {\n    public User GetUser(int id) { return null; }\n}";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/Service.cs", TsLanguage::CSharp, &config);
        assert!(!result.nodes.is_empty(), "Should extract at least file node");
        assert!(result.nodes.len() > 1, "Should extract content nodes");
    }

    #[test]
    fn test_ts_extract_ruby() {
        let content = "class UserService\n  def get_user(id)\n    # NOTE: Legacy API\n    @db.query(id)\n  end\nend";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "app/service.rb", TsLanguage::Ruby, &config);
        assert!(result.nodes.iter().any(|n| n.label == "UserService"));
        assert!(result.nodes.iter().any(|n| n.label == "get_user"));
    }

    #[test]
    fn test_ts_extract_php() {
        let content = "<?php\nuse App\\Models\\User;\nclass UserService {\n    public function getUser(int $id): User { return null; }\n}";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "src/Service.php", TsLanguage::Php, &config);
        assert!(!result.nodes.is_empty(), "Should extract at least file node");
        assert!(result.nodes.len() > 1, "Should extract content nodes");
    }

    #[test]
    fn test_ts_extract_go() {
        let content = "package main\nimport \"fmt\"\ntype UserService struct { db *Database }\nfunc (s *UserService) GetUser(id int) *User { return nil }";
        let config = ExtractConfig::default();
        let result = TreeSitterExtractor::extract(content, "pkg/service.go", TsLanguage::Go, &config);
        assert!(result.nodes.iter().any(|n| n.label == "UserService"));
        assert!(result.nodes.iter().any(|n| n.label == "GetUser"));
    }
}
