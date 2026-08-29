//! # context_optimizer
//!
//! The Smart Context Optimizer for Ink.
//!
//! This crate turns a [`RepositoryAnalysis`] (plus an optional
//! [`dependency_graph::models::AnalysisResult`]) into a compact, deterministic
//! context bundle for a specific developer request:
//!
//! * **gating** — binary/media assets and generated lockfiles are excluded
//!   up front so the budget is spent on real source,
//! * **selection** — files are ranked by relevance to the request query using
//!   path tokens, filename stems, module membership, language names, entry
//!   points, graph centrality and reachability, optionally filtered by a
//!   `min_relevance` threshold,
//! * **deduplication** — near-duplicate file content is collapsed with a
//!   MinHash LSH pipeline so repeated code and generated mirrors are sent once,
//! * **pruning** — an optional token/file budget is enforced, dropping the
//!   least relevant overflow with an audit trail,
//! * **estimation** — approximate token counts per file and in total,
//! * **reporting** — a JSON document and a human-readable Markdown report with
//!   selection reasons, dedup groups and reduction metrics.
//!
//! The crate is deliberately decoupled: it consumes the analysis over disk
//! reads, does not touch the MCP server or VS Code extension, and produces
//! fully serializable output with deterministically ordered collections.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod dedup;
pub mod error;
pub mod gate;
pub mod models;
pub mod optimizer;
pub mod output;
pub mod pruner;
pub mod selector;
pub mod tokens;

pub use error::{Error, Result};
pub use gate::{classify, ContentClass};
pub use models::{
    ContextRequest, DedupGroup, DedupSummary, DroppedFile, DroppedReason, FileContext,
    OptimizationMetrics, OptimizedContext, TokenSummary,
};
pub use optimizer::{optimize_context, ContextOptimizer, OptimizerConfig};
pub use output::report::render_report;
