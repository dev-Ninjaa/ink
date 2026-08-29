//! # repository_intelligence
//!
//! A high-performance, production-grade repository analysis engine that is the
//! foundation for every future Ink subsystem.
//!
//! The engine can, in one pass:
//!
//! * recursively scan repositories (git repositories, monorepos, standard
//!   applications) while skipping VCS metadata and build artifacts,
//! * detect languages, frameworks, package managers, build systems, lockfiles
//!   and configuration files,
//! * find likely entry points with confidence scores,
//! * extract import/export/module relationships without a full parser,
//! * discover logical modules (features, layers, monorepo packages),
//! * and serialize everything into a deterministic JSON document.
//!
//! ## Quick start
//!
//! ```no_run
//! use repository_intelligence::analyze_repository;
//! use repository_intelligence::output::json::to_json;
//!
//! let analysis = analyze_repository("path/to/repo")?;
//! let json = to_json(&analysis)?;
//! # Ok::<(), repository_intelligence::Error>(())
//! ```
//!
//! ## Design notes
//!
//! * No tree-sitter — import extraction uses layered regular expressions.
//! * Traversal and analysis are parallelised with rayon; every collection in
//!   the output is deterministically ordered so results are reproducible.
//! * The crate is intentionally independent of the future MCP server and
//!   VS Code extension; those consume [`RepositoryAnalysis`] over JSON.
//!
//! ## Feature flags
//!
//! * `heap-profiling` — enables `dhat`-based heap accounting inside the
//!   benchmark harness.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod analyzer;
pub mod error;
pub mod models;
pub mod output;
pub mod util;

pub use analyzer::{analyze_repository, Analyzer, AnalyzerConfig};
pub use error::{Error, Result};
pub use models::{
    AnalysisSummary, Ecosystem, EntryPoint, FileEntry, Framework, Language, Module, ModuleKind,
    PerformanceMetrics, ProjectMetadata, Relationship, RelationshipKind, RepositoryAnalysis,
};
pub use output::report::{render_benchmark_report, render_report};
