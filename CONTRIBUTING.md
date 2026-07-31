# Contributing to Graphify Pro

Thanks for your interest in contributing! Graphify Pro is a Rust reimagining of the Graphify codebase knowledge graph tool — built for speed, low memory, and broad language support.

## Getting Started

1. **Install Rust** (stable 1.82+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **Clone the repo**: `git clone https://github.com/graphify-pro/graphify-pro.git && cd graphify-pro`
3. **Build**: `cargo build --release`
4. **Run tests**: `cargo test --workspace`

## Project Structure

```
crates/
├── graphify-core/      # Core types: GraphNode, GraphEdge, GraphDB, NodeType
├── graphify-detect/    # File discovery & language detection (36 languages)
├── graphify-extract/   # AST extraction via tree-sitter (27 langs) + regex fallback (9)
├── graphify-build/     # Graph construction, manifest caching, manifest introspection
├── graphify-cluster/   # Community detection (Louvain), god nodes, impact analysis
├── graphify-analyze/   # Code quality: dead code, circular deps, architecture detection
├── graphify-export/    # JSON, HTML/D3, Mermaid, SVG, Neo4j CSV, Obsidian wiki
├── graphify-watch/     # File system watcher for incremental rebuilds
└── graphify-cli/       # CLI binary: build, serve, prs, benchmark, hook, global-graph
```

## Development Workflow

```bash
# Check compilation
cargo check

# Run all tests
cargo test --workspace

# Format code
cargo fmt

# Lint
cargo clippy --workspace -- -D warnings

# Run on a test project
cargo run -- build /path/to/your/project
```

## Adding a New Tree-Sitter Language

1. Add the tree-sitter crate to `crates/graphify-extract/Cargo.toml`
2. Add a variant to the `TsLanguage` enum in `crates/graphify-extract/src/lib.rs`
3. Add a `LanguageConfig` in the `language_config()` match in `tree_sitter.rs`
4. Add a test in the test module

## Adding a New Regex Fallback Language

1. Add a variant to the `Language` enum in `crates/graphify-detect/src/lib.rs`
2. Add the language detection pattern in `detect_language()`
3. Add file extension/pattern to `source_file_patterns()`
4. The generic regex extractor in `graphify-extract` will handle it automatically

## Pull Request Guidelines

- Keep PRs focused on a single feature or fix
- Add tests for new functionality
- Run `cargo fmt` and `cargo clippy` before submitting
- Update `comparison.md` if the PR addresses a feature gap vs original Graphify

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
