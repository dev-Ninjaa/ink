//! The analysis pipeline: scan → detect → extract → summarise.

pub mod entrypoint_detector;
pub mod framework_detector;
pub mod import_extractor;
pub mod language_detector;
pub mod metadata_detector;
pub mod module_detector;
pub mod scanner;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use dashmap::DashMap;
use rayon::prelude::*;

use crate::error::Result;
use crate::models::{
    AnalysisSummary, EntryPoint, FileEntry, PerformanceMetrics, Relationship, RepositoryAnalysis,
};
use crate::util;

use self::entrypoint_detector::EntryPointDetector;
use self::framework_detector::FrameworkDetector;
use self::import_extractor::ImportExtractor;
use self::language_detector::LanguageDetector;
use self::metadata_detector::MetadataDetector;
use self::module_detector::ModuleDetector;
use self::scanner::{RepositoryScanner, ScanOptions, ScanResult};

/// Configuration for a full analysis run.
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Scanner options (traversal backend, ignored dirs, gitignore).
    pub scan_options: ScanOptions,
    /// Maximum source file size (bytes) considered by the import extractor.
    pub max_source_bytes: u64,
    /// Minimum number of files a directory must contain to be a module.
    pub min_module_files: usize,
    /// Custom JS/TS specifier aliases overriding the defaults.
    pub custom_js_path_aliases: Vec<(String, String)>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        AnalyzerConfig {
            scan_options: ScanOptions::default(),
            max_source_bytes: util::DEFAULT_MAX_SOURCE_BYTES,
            min_module_files: 1,
            custom_js_path_aliases: Vec::new(),
        }
    }
}

/// The repository intelligence engine entry point.
///
/// ```no_run
/// use repository_intelligence::analyzer::Analyzer;
///
/// let analysis = Analyzer::with_defaults().analyze("path/to/repo")?;
/// # Ok::<(), repository_intelligence::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Analyzer {
    config: AnalyzerConfig,
}

impl Analyzer {
    /// Construct an analyzer with custom configuration.
    pub fn new(config: AnalyzerConfig) -> Self {
        Analyzer { config }
    }

    /// Construct an analyzer with default configuration.
    pub fn with_defaults() -> Self {
        Analyzer {
            config: AnalyzerConfig::default(),
        }
    }

    /// Access the effective configuration.
    pub fn config(&self) -> &AnalyzerConfig {
        &self.config
    }

    /// Run only the scanning phase.
    pub fn scan(&self, root: impl AsRef<Path>) -> Result<ScanResult> {
        let scanner = RepositoryScanner::new(self.config.scan_options.clone());
        scanner.scan(root)
    }

    /// Run a full analysis of a repository.
    pub fn analyze(&self, root: impl AsRef<Path>) -> Result<RepositoryAnalysis> {
        let started = Instant::now();
        let scan = self.scan(&root)?;
        let scan_duration = started.elapsed();

        let analysis_started = Instant::now();

        // Language histogram (parallel).
        let detector = LanguageDetector;
        let counts = DashMap::new();
        scan.files
            .par_iter()
            .filter_map(|file| detector.detect(&file.rel))
            .for_each(|language| {
                *counts.entry(language).or_insert(0) += 1;
            });
        let languages: BTreeMap<_, _> = counts.into_iter().collect();

        // Frameworks, metadata, entry points.
        let frameworks = FrameworkDetector.detect(&scan, self.config.max_source_bytes);
        let metadata = MetadataDetector.detect(&scan);
        let entry_points: Vec<EntryPoint> = EntryPointDetector.detect(&scan);

        // Import/export relationships.
        let extractor = ImportExtractor::new(
            self.config.max_source_bytes,
            self.config.custom_js_path_aliases.clone(),
        );
        let relationships: Vec<Relationship> = extractor.extract(&scan);

        // Modules.
        let modules = ModuleDetector {
            min_files: self.config.min_module_files,
        }
        .detect(&scan);

        let analysis_duration = analysis_started.elapsed();
        let total_duration = started.elapsed();

        let files: Vec<FileEntry> = scan
            .files
            .iter()
            .map(|file| FileEntry {
                path: util::forward_slashes(&file.rel),
                size: file.size,
                language: detector.detect(&file.rel),
            })
            .collect();
        let directories: Vec<String> = scan
            .directories
            .iter()
            .map(|dir| util::forward_slashes(dir))
            .collect();
        let project_roots: Vec<String> = scan
            .project_roots
            .iter()
            .map(|root| util::forward_slashes(root))
            .collect();

        let total_seconds = total_duration.as_secs_f64().max(f64::MIN_POSITIVE);
        let files_per_second = scan.stats.files as f64 / total_seconds;

        let summary = AnalysisSummary {
            files: scan.stats.files,
            directories: scan.stats.directories,
            project_roots: scan.project_roots.len(),
            bytes: scan.stats.bytes,
        };

        let performance = PerformanceMetrics {
            scan_duration_ms: scan_duration.as_secs_f64() * 1000.0,
            analysis_duration_ms: analysis_duration.as_secs_f64() * 1000.0,
            total_duration_ms: total_duration.as_secs_f64() * 1000.0,
            files_per_second,
        };

        let root_path = root.as_ref();

        Ok(RepositoryAnalysis {
            root: root_path.to_string_lossy().into_owned(),
            analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
            summary,
            performance,
            languages,
            frameworks,
            metadata,
            project_roots,
            entry_points,
            modules,
            relationships,
            files,
            directories,
        })
    }
}

/// Analyse a repository with a default-configuration analyzer.
///
/// ```no_run
/// let m = repository_intelligence::analyze_repository("path/to/repo")?;
/// # Ok::<(), repository_intelligence::Error>(())
/// ```
pub fn analyze_repository(root: impl AsRef<Path>) -> Result<RepositoryAnalysis> {
    Analyzer::with_defaults().analyze(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Language;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn full_pipeline_analysis() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "mod routes;\nuse crate::routes::user;");
        write(root, "src/routes/mod.rs", "pub mod user;");
        write(root, "src/routes/user.rs", "pub fn handle(){}");
        write(root, "README.md", "# demo");

        let analysis = Analyzer::with_defaults().analyze(root).unwrap();
        assert_eq!(analysis.summary.files, 4);
        assert_eq!(analysis.languages[&Language::Rust], 3);
        assert_eq!(analysis.languages[&Language::Markdown], 1);
        assert_eq!(analysis.entry_points.len(), 1);
        assert_eq!(analysis.entry_points[0].path, "src/main.rs");
        assert!(!analysis.relationships.is_empty());
        assert!(!analysis.modules.is_empty());
        assert!(analysis.performance.files_per_second > 0.0);
        assert_eq!(analysis.analyzer_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn repeated_analysis_is_deterministic() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[package]\nname=\"x\"");
        write(root, "src/main.rs", "mod m;\nuse crate::m::f;");
        write(root, "src/m.rs", "pub fn f(){}");

        let mut a = Analyzer::with_defaults().analyze(root).unwrap();
        let b = Analyzer::with_defaults().analyze(root).unwrap();

        // Timing metrics legitimately differ between runs; pin them so the
        // structural output can be compared byte-for-byte.
        a.performance = b.performance;

        let aj = serde_json::to_string(&a).unwrap();
        let bj = serde_json::to_string(&b).unwrap();
        assert_eq!(aj, bj);
    }

    #[test]
    fn invalid_root_errors() {
        let err = Analyzer::with_defaults()
            .analyze(Path::new("not/a/repo/at/all"))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "repository root `not/a/repo/at/all` does not exist or is not a directory"
        );
    }
}
