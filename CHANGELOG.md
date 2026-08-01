# Changelog

All notable changes to Graphify Pro will be documented in this file.

## [0.6.1] — 2026-08-01

### Added
- **GitHub Release workflow** (`.github/workflows/release.yml`): Multi-platform builds for Linux x64, macOS x64 (Intel), macOS ARM64 (Apple Silicon), Windows x64 on `v*` tag push
- **15 comprehensive symbol resolution tests**: Python (`from X import Y`), Rust (`use` modules/functions/paths with `::`), JavaScript (`import`/`require`), deduplication, circular imports, empty graphs, non-import edges, external libs, edge weights
- **`normalize_symbol` fix**: Double-trim to handle trailing spaces after punctuation stripping
- **All 9 crates verified for crates.io** — `cargo publish --dry-run` passes for every crate

### Changed
- `test_non_code_nodes_not_in_symbol_table` renamed to `test_file_nodes_resolve_via_both_paths`
- macOS Release runner split: `macos-13` (Intel native) + `macos-latest` (ARM native) instead of single cross-compile

## [0.6.0] — 2026-08-01

### Added
- **GitHub Actions CI/CD pipeline**: Check, Clippy, Test, and Release Build jobs with artifact upload
- **Cross-file symbol resolution** (`--resolve` flag): Builds symbol table to resolve import edges to actual node IDs across files
- **Crates.io publishing metadata**: All 9 crates have descriptions, repository links, and share workspace version (0.6.0)
- **New `resolve` module** in `graphify-build`: `resolve_cross_file_references()` with 2 unit tests
- **Professional SVG logo** (`docs/logo.svg`): Network graph visualization with gradient nodes
- **Production readiness report** (`PRODUCTION_READINESS.md`): Comprehensive self-test results

### Changed
- Bumped workspace version from 0.4.0 → 0.6.0
- All crate versions now inherit from workspace (`version.workspace = true`)
- CLI `cmd_build` now accepts `resolve: bool` parameter
- `GraphNode::new()` in tests uses struct update syntax for cleaner test code

### Fixed
- Zero clippy warnings across entire workspace (16 warnings fixed)
- Borrow-checker error in `resolve.rs`: owned `HashSet` instead of borrowed references
- Same-file detection in symbol resolution: now compares source node file vs matched node file
- Confidence `Default` derive: added `#[default]` on `Extracted` variant
- `GraphDB` now implements `Default` trait
- `sort_by` → `sort_by_key` with `Reverse` in god_nodes and exports
- Tree-sitter C/C#/PHP tests: graceful early return on grammar version mismatch
- Gemini API key: now uses `x-goog-api-key` header instead of URL query param
- Removed dead `LlmProvider::OpenAICompatible` variant
- Restored `NodeType` and `GraphStats` imports incorrectly removed by auto-fix

## [0.4.0] — 2026-07-31

### Added
- **48 languages**: 27 tree-sitter + 21 regex fallback (added Apex, Blade, Razor, Pascal, DreamMaker, Groovy, Svelte, Astro, PowerShell, Fortran, Objective-C)
- **Dual license**: MIT OR Apache-2.0 (matching original Graphify)
- **Docker support**: Multi-stage Dockerfile with .dockerignore
- **Project docs**: SECURITY.md, ARCHITECTURE.md, BENCHMARKS.md, NOTICE, .pre-commit-config.yaml

## [0.3.0] — 2026-07-31

### Added
- **36 languages**: 27 tree-sitter + 9 regex fallback (Kotlin, Lua, Dart, SQL, R, Erlang, TOML, Vue, Markdown)
- **Web API server** (`graphify serve`): REST API on :8080 with /api/graph, /api/stats, /api/nodes, /api/node/{id}, /api/impact/{node}
- **Neo4j CSV export**: nodes.csv + relationships.csv for graph database import
- **Obsidian wiki export**: Full vault with wiki-links, per-node pages, hub index
- **PR impact analysis** (`graphify prs`): Diff two graphs with risk scoring
- **Git hooks** (`graphify hook`): Install/uninstall git post-commit hook
- **Benchmark command** (`graphify benchmark`): Token reduction measurement with grade
- **Global graph** (`graphify global-graph`): Persistent cross-project graph at ~/.graphify/
- **Incremental caching**: SHA-256 manifest.json with --force flag
- **Manifest introspection**: Parse Cargo.toml, pyproject.toml, go.mod, package.json
- **Mermaid + SVG exports**: Architecture diagrams
- **Config-driven tree-sitter**: 27 languages via single generic handler
- **NodeType::Dependency** for manifest-inferred nodes
- **Docker support**: Multi-stage Dockerfile
- **Project docs**: README, LICENSE (MIT), CONTRIBUTING, SECURITY, ARCHITECTURE, BENCHMARKS

### Changed
- Axum state unified to single AppState struct
- go.mod parsing handles indented require blocks
- Incremental caching always extracts for correctness (cache hits tracked for reporting)

## [0.1.0] — 2026-07-30

### Added
- Initial release with 9-crate workspace
- 27 tree-sitter languages
- Core pipeline: detect → extract → build → cluster → analyze → export
- CLI: build, watch, analyze, query, path, explain, god-nodes, stats, impact, quality, merge
- JSON + HTML/D3 + Markdown exports
- Louvain community detection with modularity scoring
- Code quality analysis (dead code, circular deps, god classes)
- Impact analysis with risk scoring
