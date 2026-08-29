//! Top-level repository analysis model and its sub-models.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::framework::Framework;
use super::language::Language;
use super::module::Module;
use super::relationship::Relationship;

/// Coarse file counts for quick summarisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisSummary {
    /// Number of files discovered.
    pub files: u64,
    /// Number of directories discovered.
    pub directories: u64,
    /// Number of project roots discovered.
    pub project_roots: usize,
    /// Total bytes of scanned files.
    pub bytes: u64,
}

/// Timing and throughput metrics for a single analysis run.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PerformanceMetrics {
    /// Wall-clock time spent walking the repository, in milliseconds.
    pub scan_duration_ms: f64,
    /// Wall-clock time spent analysing files, in milliseconds.
    pub analysis_duration_ms: f64,
    /// Total wall-clock duration of the analysis, in milliseconds.
    pub total_duration_ms: f64,
    /// Throughput: files analysed per second.
    pub files_per_second: f64,
}

/// Tooling metadata discovered in a repository (package managers, build
/// systems, lockfiles, configuration and CI files).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectMetadata {
    /// Inferred package managers (e.g. `pnpm`, `cargo`, `poetry`).
    pub package_managers: Vec<String>,
    /// Detected build systems (e.g. `make`, `cmake`, `gradle`).
    pub build_systems: Vec<String>,
    /// Repository-relative paths of lockfiles.
    pub lockfiles: Vec<String>,
    /// Repository-relative paths of notable configuration files.
    pub config_files: Vec<String>,
    /// Repository-relative paths of manifest/project definition files.
    pub manifests: Vec<String>,
    /// Detected CI systems (e.g. `github_actions`).
    pub ci: Vec<String>,
    /// Whether any Docker-related file (Dockerfile/compose) exists.
    pub has_docker: bool,
}

/// A probable application entry point with a confidence score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EntryPoint {
    /// Repository-relative path of the entry point file.
    pub path: String,
    /// Confidence in `0.0..=1.0`.
    pub confidence: f64,
    /// Machine-readable label describing which heuristic fired.
    pub heuristic: String,
}

/// A single discovered file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileEntry {
    /// Repository-relative path of the file.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// Detected language, if any.
    pub language: Option<Language>,
}

/// Complete, self-contained output of one repository analysis.
///
/// This type is fully serializable and stable across runs (all collections
/// are deterministically ordered), which makes it a safe contract for future
/// Ink subsystems (Dependency Graph Builder, Context Optimizer, MCP server,
/// VS Code extension).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepositoryAnalysis {
    /// Absolute path of the analysed repository root.
    pub root: String,
    /// Version of the analyzer that produced this output.
    pub analyzer_version: String,
    /// Coarse counters.
    pub summary: AnalysisSummary,
    /// Timing/throughput measurements.
    pub performance: PerformanceMetrics,
    /// Occurrences per detected language (deterministically ordered).
    pub languages: BTreeMap<Language, usize>,
    /// Detected frameworks (deterministically ordered).
    pub frameworks: Vec<Framework>,
    /// Tooling metadata.
    pub metadata: ProjectMetadata,
    /// Discovered project roots.
    pub project_roots: Vec<String>,
    /// Probable entry points, sorted by confidence.
    pub entry_points: Vec<EntryPoint>,
    /// Logical modules.
    pub modules: Vec<Module>,
    /// File-to-file import/export edges.
    pub relationships: Vec<Relationship>,
    /// Every discovered file.
    pub files: Vec<FileEntry>,
    /// Every discovered directory.
    pub directories: Vec<String>,
}
