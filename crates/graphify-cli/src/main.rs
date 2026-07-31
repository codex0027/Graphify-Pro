//! # Graphify Pro CLI
//!
//! The command-line interface for Graphify Pro — a knowledge graph builder
//! for codebases. Run `graphify` to analyze your project.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Graphify Pro — Build queryable knowledge graphs from your codebase.
#[derive(Parser)]
#[command(name = "graphify", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a knowledge graph from the current project
    Build {
        /// Project root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "graphify-out")]
        output: PathBuf,

        /// Maximum file size in MB
        #[arg(long, default_value = "10")]
        max_file_size: u64,

        /// Force full rebuild (ignore cache)
        #[arg(long)]
        force: bool,
    },

    /// Watch the project for changes and update the graph automatically
    Watch {
        /// Project root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "graphify-out")]
        output: PathBuf,
    },

    /// Analyze an existing graph.json
    Analyze {
        /// Path to graph.json
        #[arg(default_value = "graphify-out/graph.json")]
        graph: PathBuf,

        /// Number of god nodes to show
        #[arg(long, default_value = "10")]
        top: usize,

        /// Check for code quality issues
        #[arg(long)]
        quality: bool,
    },

    /// Query the knowledge graph
    Query {
        /// Natural language or structured query
        question: String,

        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },

    /// Find shortest path between two nodes in the graph
    Path {
        /// Source node
        source: String,

        /// Target node
        target: String,

        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },

    /// Explain a node and its connections
    Explain {
        /// The node to explain
        node: String,

        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },

    /// List god nodes (architectural hubs)
    GodNodes {
        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,

        /// How many to show
        #[arg(long, default_value = "10")]
        top: usize,
    },

    /// Show graph statistics
    Stats {
        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },

    /// Impact analysis — see what changing a node affects
    Impact {
        /// Node(s) to analyze
        nodes: Vec<String>,

        /// Traversal depth
        #[arg(short, long, default_value = "3")]
        depth: usize,

        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },

    /// Check for code quality issues
    Quality {
        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },

    /// Merge two graph.json files for multi-repo analysis
    Merge {
        /// Graph files to merge
        graphs: Vec<PathBuf>,

        /// Output path
        #[arg(short, long, default_value = "graphify-out/merged-graph.json")]
        output: PathBuf,
    },

    /// Start web server to browse the knowledge graph
    Serve {
        /// Path to graph.json
        #[arg(short, long, default_value = "graphify-out/graph.json")]
        graph: PathBuf,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Compare two graphs (e.g., for PR impact analysis)
    Prs {
        /// Base graph (before changes)
        base: PathBuf,

        /// Head graph (after changes)
        head: PathBuf,
    },

    /// Manage the global knowledge graph (~/.graphify/global-graph.json)
    GlobalGraph {
        #[command(subcommand)]
        action: GlobalAction,
    },

    /// Install git post-commit hook for automatic graph updates
    Hook {
        /// Uninstall the hook
        #[arg(long)]
        uninstall: bool,
    },

    /// Benchmark token reduction — compare raw code size vs graph size
    Benchmark {
        /// Project root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Path to existing graph.json (skips rebuild if provided)
        #[arg(short, long)]
        graph: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum GlobalAction {
    /// Show global graph stats
    Stats,
    /// Merge a project graph into the global graph
    Merge {
        /// Path to graph.json
        #[arg(default_value = "graphify-out/graph.json")]
        graph: PathBuf,
    },
    /// Reset/clear the global graph
    Reset,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { path, output, max_file_size, force } => {
            cmd_build(&path, &output, max_file_size, force)
        }
        Commands::Watch { path, output } => cmd_watch(&path, &output),
        Commands::Analyze { graph, top, quality } => cmd_analyze(&graph, top, quality),
        Commands::Query { question, graph } => cmd_query(&question, &graph),
        Commands::Path { source, target, graph } => cmd_path(&source, &target, &graph),
        Commands::Explain { node, graph } => cmd_explain(&node, &graph),
        Commands::GodNodes { graph, top } => cmd_god_nodes(&graph, top),
        Commands::Stats { graph } => cmd_stats(&graph),
        Commands::Impact { nodes, depth, graph } => cmd_impact(&nodes, depth, &graph),
        Commands::Quality { graph } => cmd_quality(&graph),
        Commands::Merge { graphs, output } => cmd_merge(&graphs, &output),
        Commands::Serve { graph, port } => cmd_serve(&graph, port),
        Commands::Prs { base, head } => cmd_prs(&base, &head),
        Commands::GlobalGraph { action } => cmd_global_graph(action),
        Commands::Hook { uninstall } => cmd_hook(uninstall),
        Commands::Benchmark { path, graph } => cmd_benchmark(&path, graph.as_ref()),
    }
}

// ── Command Implementations ───────────────────────────────────────────────────

fn cmd_build(path: &PathBuf, output: &PathBuf, max_file_size: u64, force: bool) -> Result<(), anyhow::Error> {
    println!("🔬 Graphify Pro — Building Knowledge Graph");
    println!("   Project: {}", path.display());
    println!("   Output:  {}", output.display());
    println!();

    let start = std::time::Instant::now();

    // Load incremental cache manifest
    let manifest_path = output.join("manifest.json");
    let mut manifest = graphify_build::BuildManifest::load(&manifest_path);
    let mut cache_hits = 0;

    // Detect files
    let detection = graphify_detect::detect_files(path, max_file_size, false)?;
    println!("📁 Found {} files", detection.total_included);

    // Detect manifest files (Cargo.toml, pyproject.toml, go.mod, etc.)
    let manifest_deps = graphify_build::extract_manifest_deps(path);
    if !manifest_deps.is_empty() {
        println!("📦 Found {} manifest dependencies", manifest_deps.len());
    }

    // Extract code files
    let config = graphify_extract::ExtractConfig {
        max_workers: num_cpus::get(),
        extract_rationale: true,
        code_only: true,
        root: path.clone(),
    };

    let mut extractions = Vec::new();
    if let Some(code_files) = detection.files.get(&graphify_detect::FileCategory::Code) {
        for file in code_files {
            let rel_path = file.relative_path.to_string_lossy().to_string();
            match std::fs::read_to_string(&file.path) {
                Ok(content) => {
                    // Incremental cache check — always extract for correctness,
                    // but track cache hits for reporting
                    let is_cached = !force && manifest.is_unchanged(&rel_path, &content);
                    if is_cached {
                        cache_hits += 1;
                    }
                    let result = graphify_extract::RegexExtractor::extract_file(
                        &content,
                        &rel_path,
                        &config,
                    );
                    manifest.update(rel_path, &content, result.language.clone());
                    extractions.push(result);
                }
                Err(e) => {
                    eprintln!("  ⚠️ Failed to read {}: {}", file.path.display(), e);
                }
            }
        }
    }

    if cache_hits > 0 {
        println!("⚡ {} files unchanged (cached)", cache_hits);
    }
    println!("📝 Extracted {} files", extractions.len());

    // Add manifest deps as nodes
    for dep in &manifest_deps {
        let mut nodes = vec![
            graphify_core::node::GraphNode::new(
                &format!("dep_{}", dep.name.replace(['-', '.'], "_")),
                &dep.name,
                graphify_core::node::NodeType::Dependency,
            ),
        ];
        if let Some(ref ver) = dep.version {
            nodes[0].metadata = Some(serde_json::json!({"version": ver}));
        }
        extractions.push(graphify_extract::ExtractionResult {
            file_path: format!("manifest:{}", dep.manifest),
            nodes,
            edges: vec![],
            language: "Manifest".into(),
            errors: vec![],
        });
    }

    // Build graph
    let mut kg = graphify_build::build_graph(&extractions, &path.to_string_lossy());
    println!("🧩 Built graph: {} nodes, {} edges", kg.stats.node_count, kg.stats.edge_count);

    // Infer additional edges
    let inferred = graphify_build::infer_edges(&kg.nodes, &mut kg.edges);
    kg.stats.edge_count = kg.edges.len();
    if inferred > 0 {
        println!("🔗 Inferred {} additional edges", inferred);
    }

    // Detect communities
    let mut communities = graphify_cluster::detect_communities(&kg);
    graphify_cluster::label_communities_heuristic(&mut communities, &kg.nodes);
    kg.communities = communities.clone();
    kg.stats.community_count = communities.len();
    println!("🏘️  Detected {} communities", communities.len());

    // Analyze
    let gods = graphify_analyze::god_nodes(&kg, 10);
    let metrics = graphify_analyze::compute_metrics(&kg);

    // Save manifest for next incremental build
    manifest.project_root = path.to_string_lossy().to_string();
    if let Err(e) = manifest.save(&manifest_path) {
        eprintln!("  ⚠️ Failed to save manifest: {}", e);
    }

    // Export
    graphify_export::export_all(&kg, &communities, &gods, &metrics, output)?;

    // Print summary
    println!();
    println!("✅ Done in {:.1}s", start.elapsed().as_secs_f64());
    println!();
    println!("═══ Architecture Summary ═══");
    println!("  Total Nodes:       {}", kg.stats.node_count);
    println!("  Total Edges:       {}", kg.stats.edge_count);
    println!("  Communities:       {}", communities.len());
    println!("  Graph Density:     {:.4}", kg.stats.density);
    if let Some(ref lang) = kg.metadata.primary_language {
        println!("  Primary Language:   {}", lang);
    }

    if !gods.is_empty() {
        println!();
        println!("  🏛️  Top Hub Nodes:");
        for (_, label, degree) in &gods[..gods.len().min(5)] {
            println!("    - {} ({} edges)", label, degree);
        }
    }

    Ok(())
}

fn cmd_watch(path: &PathBuf, output: &PathBuf) -> Result<(), anyhow::Error> {
    let config = graphify_watch::WatchConfig {
        root: path.clone(),
        output_dir: output.clone(),
        ..Default::default()
    };

    let mut watcher = graphify_watch::FileWatcher::new(config);

    println!("🔍 Building initial graph...");
    watcher.full_build()?;
    println!();
    println!("👁️  Watching for changes in {}...", path.display());
    println!("   Press Ctrl+C to stop.");

    let rx = watcher.start()?;

    for changes in rx {
        println!();
        println!("🔄 Change detected in {} files", changes.changed_files.len());
        for f in &changes.changed_files {
            println!("   - {}", f.display());
        }

        match watcher.incremental_update(&changes) {
            Ok(_) => println!("✅ Graph updated"),
            Err(e) => eprintln!("❌ Failed to update: {}", e),
        }
    }

    Ok(())
}

fn cmd_analyze(graph_path: &PathBuf, top: usize, quality: bool) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;

    println!("🔬 Graphify Pro — Analysis");
    println!();

    // God nodes
    let gods = graphify_analyze::god_nodes(&kg, top);
    println!("🏛️  Top {} God Nodes:", top);
    for (i, (_, label, degree)) in gods.iter().enumerate() {
        println!("  {}. {} — {} connections", i + 1, label, degree);
    }

    // Architecture style
    let analysis = graphify_analyze::analyze_architecture(&kg);
    println!();
    println!("🏗️  Architecture Style: {}", analysis.architecture_style.as_deref().unwrap_or("Unknown"));
    println!("💚 Health Score: {:.1}%", analysis.health_score * 100.0);

    // Quality issues
    if quality {
        println!();
        let issues = graphify_analyze::detect_quality_issues(&kg);
        if issues.is_empty() {
            println!("✅ No code quality issues detected!");
        } else {
            println!("⚠️  Code Quality Issues:");
            for issue in &issues {
                println!(
                    "  🔴 {} — {} (severity: {:.1})",
                    issue.issue_type.label(),
                    issue.description,
                    issue.severity
                );
                if let Some(ref suggestion) = issue.suggestion {
                    println!("     💡 Fix: {}", suggestion);
                }
            }
        }
    }

    Ok(())
}

fn cmd_query(question: &str, graph_path: &PathBuf) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let db = graphify_core::graph::GraphDB::from_knowledge_graph(&kg);

    // Try to find nodes matching the question
    let results = db.find_nodes(question);

    if results.is_empty() {
        println!("🔍 No nodes found matching '{}'", question);

        // Try keyword search
        let keywords: Vec<&str> = question.split_whitespace().collect();
        let mut all_results = Vec::new();
        for kw in &keywords {
            let r = db.find_nodes(kw);
            all_results.extend(r);
        }

        if all_results.is_empty() {
            println!("   Try a different query or check the graph report.");
        } else {
            println!("🔍 Found {} nodes via keyword search:", all_results.len());
            for (_id, node) in all_results.iter().take(20) {
                println!("   - {} ({})", node.label, node.node_type.label());
            }
        }
    } else {
        println!("🔍 Found {} matching nodes:", results.len());
        for (_id, node) in results.iter().take(20) {
            println!("   - {} ({})", node.label, node.node_type.label());
        }

        // BFS from first match
        if !results.is_empty() {
            let traverse = db.bfs_traverse(&results[0].0, 2);
            println!();
            println!("📡 Connected nodes from '{}':", results[0].1.label);
            for (id, depth) in traverse.iter().skip(1).take(15) {
                if let Some(node) = db.get_node(id) {
                    println!("   {}> {} ({})", "─".repeat(*depth), node.label, node.node_type.label());
                }
            }
        }
    }

    Ok(())
}

fn cmd_path(source: &str, target: &str, graph_path: &PathBuf) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let db = graphify_core::graph::GraphDB::from_knowledge_graph(&kg);

    // First find nodes matching the query
    let src_results = db.find_nodes(source);
    let tgt_results = db.find_nodes(target);

    if src_results.is_empty() {
        println!("❌ Source node '{}' not found", source);
        return Ok(());
    }
    if tgt_results.is_empty() {
        println!("❌ Target node '{}' not found", target);
        return Ok(());
    }

    let src_id = &src_results[0].0;
    let tgt_id = &tgt_results[0].0;

    match db.shortest_path(src_id, tgt_id) {
        Some(path) => {
            println!("🔗 Shortest path ({} hops):", path.len() - 1);
            for (i, node_id) in path.iter().enumerate() {
                if let Some(node) = db.get_node(node_id) {
                    if i > 0 {
                        print!(" → ");
                    }
                    print!("{}", node.label);
                }
            }
            println!();
        }
        None => {
            println!("🔗 No path found between '{}' and '{}'", source, target);
        }
    }

    Ok(())
}

fn cmd_explain(node_query: &str, graph_path: &PathBuf) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let db = graphify_core::graph::GraphDB::from_knowledge_graph(&kg);

    let results = db.find_nodes(node_query);

    if results.is_empty() {
        println!("❌ Node '{}' not found", node_query);
        return Ok(());
    }

    let (node_id, node) = &results[0];

    println!("═══ {} ═══", node.label);
    println!("  Type:     {}", node.node_type.label());
    println!("  ID:       {}", node_id);
    if let Some(ref file) = node.source_file {
        println!("  File:     {}", file);
    }
    if let Some(ref loc) = node.source_location {
        println!("  Location: {}", loc);
    }
    println!();

    // Find incoming edges
    let in_edges: Vec<_> = kg.edges.iter().filter(|e| &e.target == node_id).collect();
    let out_edges: Vec<_> = kg.edges.iter().filter(|e| &e.source == node_id).collect();

    println!("📥 Incoming connections ({}):", in_edges.len());
    for edge in in_edges.iter().take(10) {
        let src_node = kg.nodes.iter().find(|n| n.id == edge.source);
        if let Some(src) = src_node {
            println!("   ← {} ({}) [{}]", src.label, edge.relation.label(), edge.confidence);
        }
    }

    println!();
    println!("📤 Outgoing connections ({}):", out_edges.len());
    for edge in out_edges.iter().take(10) {
        let tgt_node = kg.nodes.iter().find(|n| n.id == edge.target);
        if let Some(tgt) = tgt_node {
            println!("   → {} ({}) [{}]", tgt.label, edge.relation.label(), edge.confidence);
        }
    }

    Ok(())
}

fn cmd_god_nodes(graph_path: &PathBuf, top: usize) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let gods = graphify_analyze::god_nodes(&kg, top);

    println!("🏛️  Top {} God Nodes (Architectural Hubs):", top);
    println!();
    for (i, (_, label, degree)) in gods.iter().enumerate() {
        let bar = "█".repeat((*degree as f64 / 10.0).min(50.0) as usize);
        println!("{:>2}. {} ({} connections)", i + 1, label, degree);
        println!("    {}", bar);
    }

    Ok(())
}

fn cmd_stats(graph_path: &PathBuf) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;

    println!("═══ Graph Statistics ═══");
    println!();
    println!("  Nodes:              {}", kg.stats.node_count);
    println!("  Edges:              {}", kg.stats.edge_count);
    println!("  Communities:        {}", kg.stats.community_count);
    println!("  Graph Density:      {:.4}", kg.stats.density);
    println!("  Average Degree:     {:.2}", kg.stats.avg_degree);
    println!("  Connected Components: {}", kg.stats.connected_components);
    println!("  Is Connected:       {}", if kg.stats.is_connected { "Yes" } else { "No" });
    println!();
    println!("  Confidence Distribution:");
    let cd = &kg.stats.confidence_distribution;
    println!("    EXTRACTED:  {}", cd.extracted);
    println!("    INFERRED:   {}", cd.inferred);
    println!("    AMBIGUOUS:  {}", cd.ambiguous);

    if let Some(ref lang) = kg.metadata.primary_language {
        println!();
        println!("  Primary Language:   {}", lang);
    }
    if !kg.metadata.languages.is_empty() {
        println!("  All Languages:      {}", kg.metadata.languages.join(", "));
    }

    Ok(())
}

fn cmd_impact(nodes: &[String], depth: usize, graph_path: &PathBuf) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let db = graphify_core::graph::GraphDB::from_knowledge_graph(&kg);

    for node_query in nodes {
        let results = db.find_nodes(node_query);
        if results.is_empty() {
            println!("❌ Node '{}' not found", node_query);
            continue;
        }

        let (node_id, node) = &results[0];
        let changed = vec![node_id.clone()];
        let analysis = db.impact_analysis(&changed, depth);

        println!("═══ Impact Analysis: {} ═══", node.label);
        println!("  Blast Radius:   {} nodes", analysis.blast_radius);
        println!("  Risk Score:     {:.2}", analysis.risk_score);
        println!();

        if !analysis.direct_impact.is_empty() {
            println!("  🔴 Directly affected:");
            for imp in &analysis.direct_impact {
                println!("     - {} ({:.0}%)", imp.label, imp.probability * 100.0);
            }
        }

        if !analysis.indirect_impact.is_empty() {
            println!();
            println!("  🟡 Indirectly affected:");
            for imp in analysis.indirect_impact.iter().take(10) {
                println!(
                    "     - {} ({} hops, {:.0}%) — {}",
                    imp.label, imp.distance, imp.probability * 100.0, imp.reason
                );
            }
            if analysis.indirect_impact.len() > 10 {
                println!("     ... and {} more", analysis.indirect_impact.len() - 10);
            }
        }

        println!();
    }

    Ok(())
}

fn cmd_quality(graph_path: &PathBuf) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let issues = graphify_analyze::detect_quality_issues(&kg);

    if issues.is_empty() {
        println!("✅ No code quality issues detected!");
        return Ok(());
    }

    println!("⚠️  Found {} code quality issues:", issues.len());
    println!();

    let mut by_type: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for issue in &issues {
        by_type
            .entry(issue.issue_type.label().to_string())
            .or_default()
            .push(issue);
    }

    for (label, group) in &by_type {
        println!("  {} ({} occurrences)", label, group.len());
        for issue in group.iter().take(3) {
            println!("    - {}", issue.description);
            if let Some(ref suggestion) = issue.suggestion {
                println!("      💡 {}", suggestion);
            }
        }
        println!();
    }

    Ok(())
}

fn cmd_merge(graphs: &[PathBuf], output: &PathBuf) -> Result<(), anyhow::Error> {
    if graphs.len() < 2 {
        anyhow::bail!("Need at least 2 graph files to merge");
    }

    let mut merged = graphify_core::graph::GraphDB::new();

    for (i, graph_path) in graphs.iter().enumerate() {
        let kg = load_graph(graph_path)?;
        let db = graphify_core::graph::GraphDB::from_knowledge_graph(&kg);

        let prefix = graph_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("repo{}", i));

        merged.merge(&db, &prefix)?;
        println!("📦 Merged: {} ({})", graph_path.display(), prefix);
    }

    let full_kg = merged.to_knowledge_graph("merged".into());
    let json = graphify_export::export_json(&full_kg)?;
    std::fs::create_dir_all(output.parent().unwrap_or(PathBuf::new().as_path()))?;
    std::fs::write(output, json)?;

    println!("✅ Merged graph written to {}", output.display());
    println!("   {} nodes, {} edges", full_kg.stats.node_count, full_kg.stats.edge_count);

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn load_graph(path: &PathBuf) -> Result<graphify_core::KnowledgeGraph, anyhow::Error> {
    let content = std::fs::read_to_string(path)?;
    let kg: graphify_core::KnowledgeGraph = serde_json::from_str(&content)?;
    Ok(kg)
}

// ── New Commands ──────────────────────────────────────────────────────────────

fn cmd_serve(graph_path: &PathBuf, port: u16) -> Result<(), anyhow::Error> {
    let kg = load_graph(graph_path)?;
    let db = std::sync::Arc::new(graphify_core::graph::GraphDB::from_knowledge_graph(&kg));
    let output_dir = graph_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();

    use axum::{
        extract::{Path, Query, State},
        response::{Html, Json},
        routing::get,
        Router,
    };
    use tower_http::services::ServeDir;
    use serde::Deserialize;

    #[derive(Clone)]
    struct AppState {
        db: std::sync::Arc<graphify_core::graph::GraphDB>,
        output_dir: std::sync::Arc<std::path::PathBuf>,
    }

    #[derive(Deserialize)]
    struct SearchQuery { q: Option<String> }

    #[derive(Deserialize)]
    struct ImpactQuery { depth: Option<usize> }

    let state = AppState {
        db: db.clone(),
        output_dir: std::sync::Arc::new(output_dir.clone()),
    };

    async fn serve_html(State(state): State<AppState>) -> Html<String> {
        let html_path = state.output_dir.join("graph.html");
        let content = std::fs::read_to_string(&html_path).unwrap_or_else(|_| "<h1>No graph built yet</h1>".into());
        Html(content)
    }

    async fn serve_graph(State(state): State<AppState>) -> Json<serde_json::Value> {
        let kg = state.db.to_knowledge_graph("api".into());
        Json(serde_json::to_value(&kg).unwrap_or_default())
    }

    async fn serve_stats(State(state): State<AppState>) -> Json<serde_json::Value> {
        let kg = state.db.to_knowledge_graph("api".into());
        Json(serde_json::json!({
            "nodes": kg.stats.node_count,
            "edges": kg.stats.edge_count,
            "communities": kg.stats.community_count,
            "density": kg.stats.density,
            "avg_degree": kg.stats.avg_degree,
            "language": kg.metadata.primary_language,
        }))
    }

    async fn search_nodes(
        State(state): State<AppState>,
        Query(q): Query<SearchQuery>,
    ) -> Json<Vec<serde_json::Value>> {
        let query = q.q.unwrap_or_default();
        let results = state.db.find_nodes(&query);
        let nodes: Vec<_> = results.iter().take(50).map(|(id, node)| {
            serde_json::json!({"id": id, "label": node.label, "type": node.node_type.label(), "god": node.is_god_node})
        }).collect();
        Json(nodes)
    }

    async fn get_node(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> Json<Option<serde_json::Value>> {
        let node = state.db.get_node(&id);
        Json(node.map(|n| serde_json::json!({
            "id": n.id, "label": n.label, "type": n.node_type.label(),
            "language": n.language, "file": n.source_file, "god": n.is_god_node
        })))
    }

    async fn impact_analysis(
        State(state): State<AppState>,
        Path(node): Path<String>,
        Query(q): Query<ImpactQuery>,
    ) -> Json<serde_json::Value> {
        let depth = q.depth.unwrap_or(3);
        let results = state.db.find_nodes(&node);
        if results.is_empty() {
            return Json(serde_json::json!({"error": "node not found"}));
        }
        let analysis = state.db.impact_analysis(&[results[0].0.clone()], depth);
        Json(serde_json::json!({
            "blast_radius": analysis.blast_radius,
            "risk_score": analysis.risk_score,
            "direct": analysis.direct_impact.iter().map(|i| serde_json::json!({"label": i.label, "prob": i.probability})).collect::<Vec<_>>(),
            "indirect": analysis.indirect_impact.iter().take(20).map(|i| serde_json::json!({"label": i.label, "distance": i.distance, "prob": i.probability})).collect::<Vec<_>>(),
        }))
    }

    let app = Router::new()
        .route("/", get(serve_html))
        .route("/api/graph", get(serve_graph))
        .route("/api/stats", get(serve_stats))
        .route("/api/nodes", get(search_nodes))
        .route("/api/node/{id}", get(get_node))
        .route("/api/impact/{node}", get(impact_analysis))
        .nest_service("/static", ServeDir::new(output_dir.clone()))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 Graphify Pro API server starting on http://{} ", addr);
    println!("   GET /           — Interactive graph visualization");
    println!("   GET /api/graph  — Full graph JSON");
    println!("   GET /api/stats  — Graph statistics");
    println!("   GET /api/nodes?q=search — Search nodes");
    println!("   GET /api/node/{{id}} — Get node details");
    println!("   GET /api/impact/{{node}}?depth=3 — Impact analysis");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        Ok::<_, anyhow::Error>(())
    })
}

fn cmd_prs(base: &PathBuf, head: &PathBuf) -> Result<(), anyhow::Error> {
    let base_kg = load_graph(base)?;
    let head_kg = load_graph(head)?;

    let base_nodes: std::collections::HashSet<&str> = base_kg.nodes.iter().map(|n| n.id.as_str()).collect();
    let head_nodes: std::collections::HashSet<&str> = head_kg.nodes.iter().map(|n| n.id.as_str()).collect();

    let added: Vec<_> = head_nodes.difference(&base_nodes).collect();
    let removed: Vec<_> = base_nodes.difference(&head_nodes).collect();
    let changed: Vec<_> = base_nodes.intersection(&head_nodes)
        .filter(|&&id| {
            let base_node = base_kg.nodes.iter().find(|n| n.id == id);
            let head_node = head_kg.nodes.iter().find(|n| n.id == id);
            base_node.map(|n| n.label.as_str()) != head_node.map(|n| n.label.as_str())
        })
        .collect();

    println!("═══ PR Impact Analysis ═══");
    println!();
    println!("  📦 Base graph: {} nodes, {} edges", base_kg.stats.node_count, base_kg.stats.edge_count);
    println!("  📦 Head graph: {} nodes, {} edges", head_kg.stats.node_count, head_kg.stats.edge_count);
    println!();
    println!("  🟢 Added:    {} nodes", added.len());
    println!("  🔴 Removed:  {} nodes", removed.len());
    println!("  🟡 Changed:  {} nodes", changed.len());

    if !added.is_empty() {
        println!();
        println!("  New nodes:");
        for &id in added.iter().take(15) {
            if let Some(node) = head_kg.nodes.iter().find(|n| &n.id == id) {
                println!("    + {} ({})", node.label, node.node_type.label());
            }
        }
        if added.len() > 15 { println!("    ... and {} more", added.len() - 15); }
    }

    if !removed.is_empty() {
        println!();
        println!("  Removed nodes:");
        for &id in removed.iter().take(15) {
            if let Some(node) = base_kg.nodes.iter().find(|n| &n.id == id) {
                println!("    - {} ({})", node.label, node.node_type.label());
            }
        }
        if removed.len() > 15 { println!("    ... and {} more", removed.len() - 15); }
    }

    // Risk score
    let total_changes = added.len() + removed.len() + changed.len();
    let total = base_nodes.len().max(1);
    let risk = (total_changes as f64 / total as f64).min(1.0);
    println!();
    println!("  🎯 Change Impact: {:.1}% of codebase", risk * 100.0);
    println!("  🏷️  Risk Level: {}", if risk > 0.2 { "🔴 HIGH" } else if risk > 0.1 { "🟡 MEDIUM" } else { "🟢 LOW" });

    Ok(())
}

fn cmd_global_graph(action: GlobalAction) -> Result<(), anyhow::Error> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let global_dir = std::path::PathBuf::from(&home).join(".graphify");
    std::fs::create_dir_all(&global_dir)?;
    let global_path = global_dir.join("global-graph.json");

    match action {
        GlobalAction::Stats => {
            if !global_path.exists() {
                println!("📭 No global graph yet. Run 'graphify global-graph merge' to create one.");
                return Ok(());
            }
            let kg = load_graph(&global_path)?;
            println!("🌍 Global Knowledge Graph");
            println!("   Path: {}", global_path.display());
            println!("   Nodes: {}", kg.stats.node_count);
            println!("   Edges: {}", kg.stats.edge_count);
            println!("   Communities: {}", kg.stats.community_count);
            if let Some(ref lang) = kg.metadata.primary_language {
                println!("   Primary Language: {}", lang);
            }
        }
        GlobalAction::Merge { graph } => {
            let incoming = load_graph(&graph)?;
            let mut existing_db = if global_path.exists() {
                graphify_core::graph::GraphDB::from_knowledge_graph(&load_graph(&global_path)?)
            } else {
                graphify_core::graph::GraphDB::new()
            };

            let incoming_db = graphify_core::graph::GraphDB::from_knowledge_graph(&incoming);
            // Actually merge: use GraphDB.merge() method
            let project_name = incoming.metadata.project_name.as_deref().unwrap_or("unknown");
            existing_db.merge(&incoming_db, project_name)?;
            let merged_kg = existing_db.to_knowledge_graph("global".into());

            let json = serde_json::to_string_pretty(&merged_kg)?;
            std::fs::write(&global_path, json)?;
            println!("🌍 Merged into global graph: {} nodes, {} edges", merged_kg.stats.node_count, merged_kg.stats.edge_count);
        }
        GlobalAction::Reset => {
            if global_path.exists() {
                std::fs::remove_file(&global_path)?;
                println!("🗑️  Global graph reset.");
            } else {
                println!("📭 No global graph to reset.");
            }
        }
    }
    Ok(())
}

fn cmd_hook(uninstall: bool) -> Result<(), anyhow::Error> {
    let hook_path = std::path::PathBuf::from(".git/hooks/post-commit");

    if uninstall {
        if hook_path.exists() {
            std::fs::remove_file(&hook_path)?;
            println!("🗑️  Git post-commit hook removed.");
        } else {
            println!("📭 No hook installed.");
        }
        return Ok(());
    }

    if !std::path::Path::new(".git").exists() {
        anyhow::bail!("Not a git repository. Run 'git init' first.");
    }

    if let Some(parent) = hook_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let script = r#"#!/bin/sh
# Graphify Pro — Auto-update knowledge graph on commit
graphify build --force 2>&1 | tail -5
echo "📊 Knowledge graph updated by Graphify Pro"
"#;
    std::fs::write(&hook_path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_path, perms)?;
    }

    println!("🔗 Git post-commit hook installed at {}", hook_path.display());
    println!("   The knowledge graph will auto-update on every commit.");
    Ok(())
}

fn cmd_benchmark(path: &PathBuf, graph_path: Option<&PathBuf>) -> Result<(), anyhow::Error> {
    println!("⚡ Graphify Pro — Token Reduction Benchmark");
    println!();

    let graph_path = match graph_path {
        Some(p) => p.clone(),
        None => {
            // Build graph first
            let output = std::path::PathBuf::from("graphify-out");
            println!("📝 Building graph first...");
            cmd_build(path, &output, 10, true)?;
            output.join("graph.json")
        }
    };

    // Count total chars in all source files
    let detection = graphify_detect::detect_files(path, 100, false)?;
    let mut total_chars = 0usize;
    let mut total_files = 0usize;
    for files in detection.files.values() {
        for file in files {
            if let Ok(content) = std::fs::read_to_string(&file.path) {
                total_chars += content.len();
                total_files += 1;
            }
        }
    }

    let graph_size = std::fs::read_to_string(&graph_path)?.len();
    let reduction_pct = if total_chars > 0 {
        ((total_chars as f64 - graph_size as f64) / total_chars as f64 * 100.0).max(0.0)
    } else {
        0.0
    };

    println!("═══ Benchmark Results ═══");
    println!();
    println!("  📁 Source files:       {}", total_files);
    println!("  📄 Raw source chars:   {} ({:.1} KB)", total_chars, total_chars as f64 / 1024.0);
    println!("  🗜️  Graph JSON size:    {} ({:.1} KB)", graph_size, graph_size as f64 / 1024.0);
    println!("  📉 Token reduction:    {:.1}%", reduction_pct);
    println!("  💰 Est. tokens saved:  ~{}", (total_chars as f64 - graph_size as f64) as u64 / 4);
    println!();
    println!("  🏆 Grade: {}", if reduction_pct > 90.0 { "🥇 Excellent" }
        else if reduction_pct > 80.0 { "🥈 Great" }
        else if reduction_pct > 60.0 { "🥉 Good" }
        else { "⚪ Needs more code" });

    Ok(())
}
