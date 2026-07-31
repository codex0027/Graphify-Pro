# 🔗 Graphify Pro

<p align="center">
  <img src="https://img.shields.io/badge/rust-stable%201.82%2B-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/version-0.5.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green" alt="Dual License">
  <img src="https://img.shields.io/badge/languages-48-brightgreen" alt="48 Languages">
  <img src="https://img.shields.io/badge/crates-9-purple" alt="9 Crates">
</p>

**Graphify Pro** is a blazing-fast Rust reimagining of [Graphify](https://graphify.net) — a tool that extracts, analyzes, and visualizes **codebase knowledge graphs**. It transforms your source code into a structured, queryable graph of nodes (files, classes, functions) and edges (imports, calls, inheritance).

> **250x faster startup** than the Python original. **50-150MB memory** vs 200-500MB. **48 languages**. **REST API**. **Neo4j/Obsidian exports**. **Multi-provider LLM pass** (OpenAI, Anthropic, Gemini, Ollama).

---

## ✨ Features

### 🔍 Extraction
- **48 languages** — 27 via full tree-sitter AST + 21 via regex/generic fallback
- **Config-driven tree-sitter** — single generic handler for 27 grammars
- **Rationale extraction** — captures `# NOTE:`, `# TODO:`, design comments
- **Incremental caching** — SHA-256 manifest.json; unchanged files skip extraction entirely (v2.0 manifest)
- **Parallel extraction** — rayon thread pool, auto-detects CPU count
- **Manifest introspection** — parses Cargo.toml, pyproject.toml, go.mod, package.json

### 🏗️ Graph Construction
- **Edge confidence** — EXTRACTED / INFERRED / AMBIGUOUS tagging
- **Transitive inference** — A→B + B→C ⇒ A→C for import chains
- **Deduplication** — node + edge dedup, dangling edge pruning
- **Multi-repo merge** — merge graphs with prefix namespacing
- **Global graph** — persistent cross-project graph at `~/.graphify/`

### 🧩 Analysis
- **Community detection** — Louvain algorithm with Newman-Girvan modularity scoring
- **God nodes** — degree + PageRank-based hub detection
- **Impact analysis** — forward BFS with risk scoring and probability weighting
- **PR impact** — `graphify prs base head` diff with change categorization
- **Code quality** — dead code detection, circular dependencies, god class identification
- **Architecture detection** — heuristic style classification
- **Path tracing + explain** — find connections between any two nodes

### 🤖 LLM Semantic Pass
- **Multi-provider** — OpenAI, Anthropic, Gemini, Ollama, or any OpenAI-compatible endpoint
- **Auto-detection** — picks provider from env vars (`ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `OPENAI_API_KEY`)
- **Community labeling** — AI-generated names + descriptions for detected clusters
- **Architecture insights** — LLM-powered architectural analysis of the codebase

### 📤 Export
| Format | Description |
|--------|-------------|
| **JSON** | Full graph serialization (`graph.json`) |
| **HTML/D3.js** | Interactive force-layout visualization with search, zoom, community coloring |
| **Markdown** | Obsidian-compatible report (`GRAPH_REPORT.md`) |
| **Mermaid** | Call-flow architecture diagram (`graph.mermaid.md`) |
| **SVG** | Vector architectural diagram (`graph.svg`) |
| **Neo4j CSV** | `nodes.csv` + `relationships.csv` for graph database import |
| **Obsidian Wiki** | Full vault with wiki-links, per-node pages, hub index |

### 🌐 Web API
```bash
graphify serve /path/to/graph.json
```
Starts a REST API on `:8080`:
- `GET /api/graph` — full knowledge graph
- `GET /api/stats` — node/edge/community counts
- `GET /api/nodes?q=search` — fuzzy node search
- `GET /api/node/{id}` — single node with edges
- `GET /api/impact/{node}?depth=3` — impact analysis
- `GET /` — interactive HTML visualization

---

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.82+ ([install](https://rustup.rs))

### Install

```bash
# Clone
git clone https://github.com/graphify-pro/graphify-pro.git
cd graphify-pro

# Build (optimized)
cargo build --release

# The binary is at target/release/graphify
```

### Basic Usage

```bash
# Extract a knowledge graph from a codebase
./target/release/graphify build ./my-project

# Output is in graphify-out/:
#   graph.json          - Full knowledge graph
#   graph.html          - Interactive D3.js visualization
#   GRAPH_REPORT.md     - Markdown report
#   graph.mermaid.md    - Mermaid call-flow diagram
#   graph.svg           - SVG architectural diagram
#   nodes.csv           - Neo4j node import
#   relationships.csv   - Neo4j edge import
#   obsidian/           - Obsidian wiki vault
```

### All Commands

```bash
# Build graph
graphify build [PATH]              # Extract knowledge graph
graphify build [PATH] --force      # Skip incremental cache, full rebuild

# Analyze
graphify god-nodes [GRAPH]         # Find hub nodes
graphify path [GRAPH] A B          # Find path between two nodes
graphify explain [GRAPH] X         # Explain a node with context
graphify impact [GRAPH] NODE       # Impact analysis (blast radius)
graphify quality [GRAPH]           # Dead code, circular deps, god classes

# Compare
graphify prs [BASE] [HEAD]         # PR impact — diff two graphs

# Serve
graphify serve [GRAPH]             # Start REST API on :8080

# Manage
graphify merge [GRAPH1] [GRAPH2]   # Merge two graphs
graphify global-graph merge [GRAPH] [NAME]  # Add to global graph
graphify global-graph stats        # Show global graph stats
graphify global-graph reset        # Clear global graph
graphify hook                      # Install git post-commit hook
graphify benchmark [PATH]          # Measure token reduction

# Watch
graphify watch [PATH]              # Auto-rebuild on file changes
```

---

## 📊 Supported Languages

### Tree-Sitter (27) — Full AST parsing
> These languages get deep semantic analysis via tree-sitter: nested types, generics, decorators, and precise location data.

Rust, Python, JavaScript, TypeScript, TSX, Go, Java, C, C++, C#, Swift, Ruby, PHP, Scala, Haskell, Julia, Bash, HCL (Terraform), Elixir, Zig, OCaml, JSON, YAML, CSS, HTML, Solidity, Verilog

### Regex Fallback (21) — Function/class/import extraction
> These languages use regex-based extraction: identifies functions, classes, and imports, but without deep AST accuracy.

Kotlin, Lua, Dart, SQL, R, Erlang, TOML, Vue, Markdown, Apex, Blade, Razor, Pascal, DreamMaker, Groovy, Svelte, Astro, PowerShell, Fortran, Objective-C, DM

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    graphify CLI                         │
│  build │ serve │ prs │ benchmark │ hook │ global-graph  │
└────────┬────────┬────────┬────────┬────────┬───────────┘
         │        │        │        │        │
    ┌────▼───┐ ┌──▼──┐ ┌───▼────┐ ┌▼─────┐ ┌▼──────┐
    │ build  │ │watch│ │analyze │ │export│ │cluster│
    └────┬───┘ └──┬──┘ └───┬────┘ └──┬───┘ └───┬───┘
         │        │        │        │          │
    ┌────▼───┐ ┌──▼────────▼────────▼──────────▼───┐
    │extract │ │          graphify-core             │
    │(27 TS) │ │  GraphDB │ Node │ Edge │ NodeType  │
    └────┬───┘ └───────────────────────────────────┘
         │
    ┌────▼────┐
    │ detect  │
    │(36 lang)│
    └─────────┘
```

**9 crates, clean separation of concerns, zero circular dependencies.**

---

## ⚡ Performance

| Metric | Graphify (Python) | Graphify Pro (Rust) |
|--------|------------------|---------------------|
| Startup | ~500ms | ~2ms (**250x faster**) |
| Memory | 200-500MB | 50-150MB |
| Parallelism | Subprocess (GIL) | Native threads (no GIL) |
| Graph engine | NetworkX | petgraph (native) |
| Binary size | N/A | ~15-25MB (release) |

---

## 🧪 Development

```bash
# Run all tests
cargo test --workspace

# Check compilation
cargo check

# Format + lint
cargo fmt
cargo clippy --workspace -- -D warnings

# Run on a test project
cargo run --release -- build ./some-project
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

---

## 🆚 Comparison with Original Graphify

Graphify Pro closes every major feature gap while adding unique advantages. *Note: language counts distinguish full tree-sitter AST parsing from regex-based extraction.*

| Feature | Graphify | Graphify Pro |
|---------|:--------:|:------------:|
| Languages | 36+ | ✅ **48** (27 AST + 21 regex) |
| Tree-sitter parsing | ✅ | ✅ 27 grammars |
| Incremental caching | ✅ mature | ✅ SHA-256 skip-extraction (v2.0 manifest) |
| Web API server | MCP only | ✅ REST + D3 HTML |
| Neo4j export | ✅ | ✅ |
| Obsidian wiki | ✅ | ✅ |
| PR impact analysis | ✅ | ✅ |
| Git hooks | ✅ | ✅ |
| Benchmark command | ✅ | ✅ |
| Manifest introspection | ✅ | ✅ |
| Global graph | ✅ | ✅ |
| Startup speed | 500ms | **2ms** |
| Memory | 200-500MB | **50-150MB** |
| AI editor hooks | 20+ | Web API (extensible) |
| Multimedia (PDF/video) | ✅ extensive | ✅ PDF extraction built-in |
| LLM semantic pass | ✅ | ✅ Multi-provider (OpenAI, Anthropic, Gemini, Ollama) |

See [comparison.md](comparison.md) for the full detailed comparison.

---

## 📄 License

MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE) and [LICENSE-APACHE](LICENSE-APACHE) for details.

---

<p align="center">
  <sub>Built with 🦀 Rust, 🌳 tree-sitter, and 📊 petgraph</sub>
</p>
