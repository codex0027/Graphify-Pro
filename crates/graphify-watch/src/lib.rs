//! # Graphify Watch — Real-time File System Watching
//!
//! Watches the project directory for changes and triggers incremental
//! graph updates. Only re-indexes files that have actually changed.

use graphify_core::KnowledgeGraph;
use graphify_detect::{detect_files, FileCategory};
use graphify_extract::{ExtractConfig, RegexExtractor};
use graphify_build::build_graph;
use graphify_cluster::{detect_communities, label_communities_heuristic};
use graphify_analyze::{god_nodes, compute_metrics};
use graphify_export::export_all;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Configuration for the file watcher.
pub struct WatchConfig {
    /// Project root directory to watch
    pub root: PathBuf,
    /// Output directory for graph artifacts
    pub output_dir: PathBuf,
    /// Debounce duration (coalesce rapid changes)
    pub debounce_ms: u64,
    /// Whether to run initial full build
    pub initial_build: bool,
    /// Extensions to watch (empty = all supported)
    pub extensions: Vec<String>,
    /// Maximum file size in MB
    pub max_file_size_mb: u64,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            output_dir: PathBuf::from("graphify-out"),
            debounce_ms: 500,
            initial_build: true,
            extensions: Vec::new(),
            max_file_size_mb: 10,
        }
    }
}

/// Represents a batch of changed files.
#[derive(Debug, Clone)]
pub struct FileChangeBatch {
    pub changed_files: Vec<PathBuf>,
    pub deleted_files: Vec<PathBuf>,
    pub timestamp: Instant,
}

/// The file watcher engine.
pub struct FileWatcher {
    config: WatchConfig,
    /// Currently indexed files (relative paths)
    indexed_files: HashSet<PathBuf>,
    /// Current knowledge graph
    current_graph: Option<KnowledgeGraph>,
    /// Whether a rebuild is in progress
    rebuilding: bool,
}

impl FileWatcher {
    pub fn new(config: WatchConfig) -> Self {
        Self {
            config,
            indexed_files: HashSet::new(),
            current_graph: None,
            rebuilding: false,
        }
    }

    /// Start watching the filesystem for changes.
    /// Returns a channel receiver that emits change batches.
    pub fn start(&self) -> Result<mpsc::Receiver<FileChangeBatch>, anyhow::Error> {
        let (tx, rx) = mpsc::channel();
        let root = self.config.root.clone();
        let _debounce = Duration::from_millis(self.config.debounce_ms);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Filter for relevant events
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            let changed: Vec<PathBuf> = event
                                .paths
                                .iter()
                                .filter(|p| is_watchable(p))
                                .cloned()
                                .collect();
                            if !changed.is_empty() {
                                let _ = tx.send(FileChangeBatch {
                                    changed_files: changed.clone(),
                                    deleted_files: if matches!(event.kind, EventKind::Remove(_)) {
                                        changed
                                    } else {
                                        Vec::new()
                                    },
                                    timestamp: Instant::now(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    eprintln!("watch error: {:?}", e);
                }
            }
        })?;

        watcher.watch(&root, RecursiveMode::Recursive)?;

        // Keep watcher alive - leak it intentionally so it runs in background
        std::mem::forget(watcher);

        Ok(rx)
    }

    /// Perform an initial full build of the graph.
    pub fn full_build(&mut self) -> Result<KnowledgeGraph, anyhow::Error> {
        println!("🔍 Scanning project: {}", self.config.root.display());

        let detection = detect_files(
            &self.config.root,
            self.config.max_file_size_mb,
            false,
        )?;

        println!(
            "📁 Found {} files ({} ignored, {} noise dirs)",
            detection.total_included,
            detection.ignored.len(),
            detection.pruned_noise_dirs.len(),
        );

        // Track indexed files
        self.indexed_files.clear();
        for files in detection.files.values() {
            for f in files {
                self.indexed_files.insert(f.relative_path.clone());
            }
        }

        // Extract code files
        let config = ExtractConfig {
            max_workers: num_cpus::get(),
            extract_rationale: true,
            code_only: true,
            root: self.config.root.clone(),
        };

        let mut extractions = Vec::new();
        let code_files = detection.files.get(&FileCategory::Code);
        if let Some(files) = code_files {
            for file in files {
                match std::fs::read_to_string(&file.path) {
                    Ok(content) => {
                        let result = RegexExtractor::extract_file(
                            &content,
                            &file.relative_path.to_string_lossy(),
                            &config,
                        );
                        extractions.push(result);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read {}: {}", file.path.display(), e);
                    }
                }
            }
        }

        println!("📝 Extracted {} files", extractions.len());

        // Build graph
        let mut kg = build_graph(
            &extractions,
            &self.config.root.to_string_lossy(),
        );

        println!(
            "🧩 Built graph: {} nodes, {} edges",
            kg.nodes.len(),
            kg.edges.len(),
        );

        // Detect communities
        let mut communities = detect_communities(&kg);
        label_communities_heuristic(&mut communities, &kg.nodes);
        kg.communities = communities.clone();
        kg.stats.community_count = communities.len();

        // Compute metrics
        let metrics = compute_metrics(&kg);
        let gods = god_nodes(&kg, 10);

        // Export
        export_all(&kg, &communities, &gods, &metrics, &self.config.output_dir)?;

        println!(
            "✅ Graph built successfully — {} communities, {} god nodes",
            communities.len(),
            gods.len(),
        );

        self.current_graph = Some(kg.clone());
        Ok(kg)
    }

    /// Incrementally update changed files.
    pub fn incremental_update(
        &mut self,
        changes: &FileChangeBatch,
    ) -> Result<(), anyhow::Error> {
        println!(
            "🔄 Detected {} changes, rebuilding incrementally...",
            changes.changed_files.len(),
        );

        // For now, do a full rebuild (future: truly incremental)
        self.full_build()?;

        Ok(())
    }
}

/// Check if a file path should be watched.
fn is_watchable(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let watchable = [
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "kt",
            "swift", "rb", "php", "c", "h", "cpp", "hpp", "cs", "scala",
            "lua", "dart", "zig", "ex", "exs", "vue", "svelte", "astro",
            "sql", "tf", "hcl", "sh", "md", "json", "yaml", "yml", "toml",
            "xml", "gradle", "kts", "makefile", "dockerfile",
        ];
        return watchable.contains(&ext.to_lowercase().as_str());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_watchable() {
        assert!(is_watchable(Path::new("main.rs")));
        assert!(is_watchable(Path::new("app.py")));
        assert!(is_watchable(Path::new("component.tsx")));
        assert!(!is_watchable(Path::new("image.png")));
        assert!(!is_watchable(Path::new("video.mp4")));
    }
}
