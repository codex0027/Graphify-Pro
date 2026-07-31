//! # Graphify Detect — File Discovery & Classification
//!
//! Scans project directories, classifies files by type, and determines which
//! files should be indexed for the knowledge graph.

use graphify_core::node::NodeType;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Classification of a detected file.
#[derive(Debug, Clone)]
pub struct DetectedFile {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Path relative to the project root
    pub relative_path: PathBuf,
    /// Detected file category
    pub category: FileCategory,
    /// Detected language (if a code file)
    pub language: Option<String>,
    /// File size in bytes
    pub size: u64,
    /// Whether the file is ignored by .gitignore
    pub is_ignored: bool,
}

/// Categories of files detected during scanning.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileCategory {
    /// Source code file
    Code,
    /// Documentation (markdown, rst, txt)
    Document,
    /// Academic paper (PDF)
    Paper,
    /// Image file
    Image,
    /// Audio/video file
    Media,
    /// Configuration file
    Config,
    /// Package manifest (Cargo.toml, package.json, etc.)
    Manifest,
    /// Database schema
    Database,
    /// Unknown/unhandled
    Other,
}

/// Language mapping based on file extensions.
pub struct LanguageDetector {
    ext_to_lang: HashMap<String, (String, FileCategory)>,
}

impl LanguageDetector {
    pub fn new() -> Self {
        let mut ext_to_lang = HashMap::new();

        // Code files
        let code_exts = [
            ("rs", "Rust"), ("py", "Python"), ("pyi", "Python"),
            ("js", "JavaScript"), ("jsx", "JavaScript"), ("cjs", "JavaScript"),
            ("ts", "TypeScript"), ("tsx", "TypeScript"), ("mts", "TypeScript"),
            ("go", "Go"), ("java", "Java"), ("kt", "Kotlin"), ("kts", "Kotlin"),
            ("swift", "Swift"), ("rb", "Ruby"), ("php", "PHP"),
            ("c", "C"), ("h", "C"), ("cpp", "C++"), ("cc", "C++"),
            ("hpp", "C++"), ("cs", "C#"), ("scala", "Scala"),
            ("lua", "Lua"), ("dart", "Dart"), ("zig", "Zig"),
            ("ex", "Elixir"), ("exs", "Elixir"), ("erl", "Erlang"),
            ("hs", "Haskell"), ("ml", "OCaml"), ("fs", "F#"),
            ("vue", "Vue"), ("svelte", "Svelte"), ("astro", "Astro"),
            ("sql", "SQL"), ("tf", "Terraform"), ("hcl", "Terraform"),
            ("sh", "Bash"), ("bash", "Bash"), ("zsh", "Bash"),
            ("ps1", "PowerShell"), ("psm1", "PowerShell"),
            ("jl", "Julia"), ("r", "R"), ("m", "Objective-C"),
            ("mm", "Objective-C++"), ("sol", "Solidity"),
            ("cls", "Apex"), ("trigger", "Apex"),
            ("v", "Verilog"), ("sv", "SystemVerilog"),
            ("vhd", "VHDL"), ("p", "Pascal"), ("pas", "Pascal"),
            ("f90", "Fortran"), ("f95", "Fortran"), ("dm", "DreamMaker"),
        ];

        for (ext, lang) in &code_exts {
            ext_to_lang.insert(ext.to_string(), (lang.to_string(), FileCategory::Code));
        }

        // Document files
        let doc_exts = [
            ("md", "Markdown"), ("mdx", "Markdown"), ("rst", "reStructuredText"),
            ("txt", "Text"), ("adoc", "AsciiDoc"), ("org", "Org"),
        ];
        for (ext, lang) in &doc_exts {
            ext_to_lang.insert(ext.to_string(), (lang.to_string(), FileCategory::Document));
        }

        // Config files
        let config_exts = [
            ("json", "JSON"), ("yaml", "YAML"), ("yml", "YAML"),
            ("toml", "TOML"), ("xml", "XML"), ("ini", "INI"),
            ("cfg", "Config"), ("conf", "Config"), ("env", "Env"),
        ];
        for (ext, lang) in &config_exts {
            ext_to_lang.insert(ext.to_string(), (lang.to_string(), FileCategory::Config));
        }

        // Manifest files
        let manifest_names = [
            "Cargo.toml", "package.json", "pyproject.toml", "setup.py",
            "go.mod", "pom.xml", "build.gradle", "build.gradle.kts",
            "Makefile", "CMakeLists.txt", "meson.build", "Dockerfile",
        ];
        for name in &manifest_names {
            ext_to_lang.insert(
                name.to_string(),
                ("Manifest".to_string(), FileCategory::Manifest),
            );
        }

        // Paper files
        ext_to_lang.insert("pdf".to_string(), ("PDF".to_string(), FileCategory::Paper));

        // Image files
        for ext in &["png", "jpg", "jpeg", "gif", "svg", "webp", "ico"] {
            ext_to_lang.insert(ext.to_string(), ("Image".to_string(), FileCategory::Image));
        }

        // Media files
        for ext in &["mp4", "mov", "avi", "wmv", "mp3", "wav", "ogg", "webm"] {
            ext_to_lang.insert(ext.to_string(), ("Media".to_string(), FileCategory::Media));
        }

        Self { ext_to_lang }
    }

    /// Detect language and category for a file path.
    pub fn detect(&self, path: &Path) -> Option<(String, FileCategory)> {
        // Check full filename first (for manifests like Cargo.toml)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(result) = self.ext_to_lang.get(name) {
                return Some(result.clone());
            }
        }

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(result) = self.ext_to_lang.get(&ext.to_lowercase()) {
                return Some(result.clone());
            }
        }

        None
    }
}

/// File detection result.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Project root path
    pub root: PathBuf,
    /// All detected files grouped by category
    pub files: HashMap<FileCategory, Vec<DetectedFile>>,
    /// Files that were explicitly ignored
    pub ignored: Vec<PathBuf>,
    /// Directories pruned as noise (.git, node_modules, etc.)
    pub pruned_noise_dirs: Vec<PathBuf>,
    /// Total files found (before filtering)
    pub total_found: usize,
    /// Total files included (after filtering)
    pub total_included: usize,
}

/// Detect files in a project directory.
pub fn detect_files(
    root: &Path,
    max_file_size_mb: u64,
    follow_symlinks: bool,
) -> Result<DetectionResult, anyhow::Error> {
    let detector = LanguageDetector::new();
    let max_size = max_file_size_mb * 1024 * 1024;
    let mut files: HashMap<FileCategory, Vec<DetectedFile>> = HashMap::new();
    let mut ignored = Vec::new();
    let mut pruned_dirs = Vec::new();
    let mut total_found = 0;
    let mut total_included = 0;

    // Noise directories to always skip
    let noise_dirs: Vec<&str> = vec![
        ".git", "node_modules", "__pycache__", ".venv", "venv",
        "target", "build", "dist", ".next", ".nuxt", ".cache",
        ".graphify-out", "graphify-out", "coverage", ".tox",
        ".mypy_cache", ".pytest_cache", ".ruff_cache",
    ];

    let mut builder = WalkDir::new(root);
    builder = builder.follow_links(follow_symlinks);

    for entry in builder {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        total_found += 1;

        let path = entry.path();

        // Skip noise directories
        if entry.file_type().is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if noise_dirs.contains(&dir_name) {
                    pruned_dirs.push(path.to_path_buf());
                    continue;
                }
            }
            continue;
        }

        // Skip hidden files (except .graphifyignore)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') && name != ".graphifyignore" && name != ".env" {
                ignored.push(path.to_path_buf());
                continue;
            }
        }

        // Check file size
        if let Ok(meta) = path.metadata() {
            if meta.len() > max_size {
                ignored.push(path.to_path_buf());
                continue;
            }
        }

        // Detect language and category
        let (language, category) = match detector.detect(path) {
            Some(result) => result,
            None => {
                // Skip binary/unrecognized files, but not unknown text files
                continue;
            }
        };

        // Compute relative path
        let relative_path = match path.strip_prefix(root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => path.to_path_buf(),
        };

        let detected = DetectedFile {
            path: path.to_path_buf(),
            relative_path,
            category: category.clone(),
            language: Some(language),
            size: path.metadata().map(|m| m.len()).unwrap_or(0),
            is_ignored: false,
        };

        files.entry(category).or_default().push(detected);
        total_included += 1;
    }

    Ok(DetectionResult {
        root: root.to_path_buf(),
        files,
        ignored,
        pruned_noise_dirs: pruned_dirs,
        total_found,
        total_included,
    })
}

/// Detect primary language from file counts.
pub fn detect_primary_language(files: &HashMap<FileCategory, Vec<DetectedFile>>) -> Option<String> {
    let mut lang_counts: HashMap<String, usize> = HashMap::new();

    if let Some(code_files) = files.get(&FileCategory::Code) {
        for file in code_files {
            if let Some(ref lang) = file.language {
                *lang_counts.entry(lang.clone()).or_default() += 1;
            }
        }
    }

    lang_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lang, _)| lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_python_file() {
        let detector = LanguageDetector::new();
        let result = detector.detect(Path::new("main.py"));
        assert!(result.is_some());
        let (lang, cat) = result.unwrap();
        assert_eq!(lang, "Python");
        assert_eq!(cat, FileCategory::Code);
    }

    #[test]
    fn test_detect_manifest() {
        let detector = LanguageDetector::new();
        let result = detector.detect(Path::new("Cargo.toml"));
        assert!(result.is_some());
        let (lang, cat) = result.unwrap();
        assert_eq!(lang, "Manifest");
        assert_eq!(cat, FileCategory::Manifest);
    }

    #[test]
    fn test_detect_rust_file() {
        let detector = LanguageDetector::new();
        let result = detector.detect(Path::new("src/main.rs"));
        assert!(result.is_some());
        let (lang, _) = result.unwrap();
        assert_eq!(lang, "Rust");
    }
}
