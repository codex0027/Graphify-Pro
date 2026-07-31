# Architecture

Graphify Pro is a 9-crate Rust workspace that transforms source code into queryable knowledge graphs.

## High-Level Pipeline

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  detect  │───▶│ extract  │───▶│  build   │───▶│ cluster  │───▶│ export   │
│ (files)  │    │ (AST)    │    │ (graph)  │    │ (Louvain)│    │ (JSON/…) │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
                                                      │
                                               ┌──────▼──────┐
                                               │   analyze   │
                                               │ (quality)   │
                                               └─────────────┘
```

## Crate Map

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| **graphify-core** | Core data model | `GraphNode`, `GraphEdge`, `GraphDB`, `KnowledgeGraph`, `NodeType`, `EdgeRelation`, `Confidence` |
| **graphify-detect** | File discovery + language ID | `detect_files()`, `FileCategory`, 36 languages |
| **graphify-extract** | AST/regex extraction | `TreeSitterExtractor` (27 langs), `RegexExtractor` (9 fallback) |
| **graphify-build** | Graph construction | `build_graph()`, `infer_edges()`, `BuildManifest`, `extract_manifest_deps()` |
| **graphify-cluster** | Community detection | `detect_communities()` (Louvain), `label_communities_heuristic()` |
| **graphify-analyze** | Code quality analysis | `god_nodes()`, `detect_quality_issues()`, `analyze_architecture()` |
| **graphify-export** | Output formats | JSON, HTML/D3, Mermaid, SVG, Neo4j CSV, Obsidian wiki |
| **graphify-watch** | File system watcher | `FileWatcher`, incremental rebuilds |
| **graphify-cli** | CLI binary | `build`, `serve`, `prs`, `benchmark`, `hook`, `global-graph`, etc. |

## Data Flow

### Extraction
1. `detect_files()` walks the project tree, classifies each file by language/category
2. For each code file, `extract_file()` dispatches to tree-sitter (27 languages) or regex (9 fallback)
3. Each `ExtractionResult` contains nodes (classes, functions, variables, rationale) and edges (imports, calls, inheritance)

### Graph Construction
1. `build_graph()` merges all `ExtractionResult`s into a single `KnowledgeGraph`
2. `infer_edges()` adds transitive imports (A→B + B→C ⇒ A→C)
3. Manifest deps (Cargo.toml, pyproject.toml) are added as `NodeType::Dependency` nodes

### Analysis
1. `detect_communities()` runs Louvain modularity optimization
2. `god_nodes()` finds architectural hubs by degree + PageRank
3. `detect_quality_issues()` finds dead code, circular dependencies, god classes

### Export
1. JSON serialization of the full `KnowledgeGraph`
2. HTML with D3.js v7 force-layout (interactive: search, zoom, community coloring)
3. Mermaid markdown for call-flow diagrams
4. SVG architectural diagrams
5. Neo4j CSV (nodes.csv + relationships.csv)
6. Obsidian wiki vault with per-node pages and wiki-links

## Design Decisions

- **petgraph over NetworkX**: Native Rust graph library, zero-overhead iteration, no Python overhead
- **Config-driven tree-sitter**: Single generic handler for all languages via `LangConfig` structs — adding a language requires no new per-language code
- **9 crates, not 1**: Clean separation of concerns; each crate can be tested, versioned, and reused independently
- **rayon over async**: CPU-bound extraction benefits more from thread pools than async I/O
- **axum for web API**: Lightweight, fast, well-typed HTTP framework for the serve command
