# Changelog

All notable changes to Graphify Pro will be documented in this file.

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
