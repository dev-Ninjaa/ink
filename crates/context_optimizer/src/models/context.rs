//! Core output models: the selected files, dropped files and the full
//! optimized context document.

use serde::{Deserialize, Serialize};

use super::dedup::DedupSummary;
use super::metrics::OptimizationMetrics;
use super::tokens::TokenSummary;

/// A single selected file within the optimized context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileContext {
    /// Repository-relative path.
    pub path: String,
    /// Detected language, when Repository Intelligence found one.
    pub language: Option<String>,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Approximate token count for this file's content.
    pub tokens: usize,
    /// Raw relevance score.
    pub score: f64,
    /// Relevance normalized to `0.0..=1.0` against the best candidate.
    pub relevance: f64,
    /// Human-readable reasons the file was selected.
    pub reasons: Vec<String>,
    /// File content read from disk, when it was readable.
    pub content: Option<String>,
}

/// Why a candidate file was excluded from the final bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroppedReason {
    /// The file was collapsed because it is a near-duplicate of a kept file.
    Duplicate,
    /// The file exceeded the requested token or file budget.
    BudgetExceeded,
    /// The file is a binary or media asset and never eligible as context.
    NonText,
    /// The file is a generated lockfile or bundle and never eligible as context.
    Generated,
    /// The file's normalized relevance fell below the requested `min_relevance`.
    LowRelevance,
}

/// A candidate file that was not selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DroppedFile {
    /// Repository-relative path.
    pub path: String,
    /// Why the file was dropped.
    pub reason: DroppedReason,
    /// Human-readable detail (budget numbers, duplicate representative).
    pub detail: String,
}

/// Complete output of one context optimization run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OptimizedContext {
    /// Absolute path of the analyzed repository root.
    pub root: String,
    /// Version of the optimizer that produced this output.
    pub optimizer_version: String,
    /// The request this context was optimized for.
    pub query: String,
    /// Selected files, ordered by relevance (highest first).
    pub selected: Vec<FileContext>,
    /// Files excluded by deduplication or budget pruning.
    pub dropped: Vec<DroppedFile>,
    /// Near-duplicate groups discovered during optimization.
    pub dedup: DedupSummary,
    /// Token accounting for the run.
    pub tokens: TokenSummary,
    /// Aggregate optimization metrics.
    pub metrics: OptimizationMetrics,
    /// Non-fatal observations (unreadable files, empty query, ...).
    pub warnings: Vec<String>,
}
