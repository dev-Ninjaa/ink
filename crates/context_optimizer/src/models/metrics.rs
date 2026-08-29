//! Aggregate optimization metrics.

use serde::{Deserialize, Serialize};

/// Aggregate metrics for a context optimization run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OptimizationMetrics {
    /// Number of candidate files after include/exclude filters.
    pub files_considered: usize,
    /// Number of files in the optimized bundle.
    pub files_selected: usize,
    /// Number of files dropped because of the token/file budget.
    pub files_dropped_budget: usize,
    /// Number of files collapsed as near-duplicates.
    pub files_dropped_duplicates: usize,
    /// Number of files dropped because they are binary or media assets.
    pub files_dropped_non_text: usize,
    /// Number of files dropped because they are generated lockfiles/bundles.
    pub files_dropped_generated: usize,
    /// Number of files dropped because their relevance was below `min_relevance`.
    pub files_dropped_low_relevance: usize,
    /// Number of files removed by include/exclude filters.
    pub files_excluded: usize,
    /// Total bytes of all candidate files.
    pub bytes_before: u64,
    /// Total bytes of the optimized bundle.
    pub bytes_after: u64,
    /// Approximate tokens of all candidate files.
    pub tokens_before: usize,
    /// Approximate tokens of the optimized bundle.
    pub tokens_after: usize,
    /// Percentage of candidate tokens removed by optimization.
    pub token_reduction_percent: f64,
    /// Fraction of candidate tokens removed (`0.0..=1.0`).
    pub redundancy_ratio: f64,
    /// Wall-clock duration of the optimization in milliseconds.
    pub duration_ms: f64,
}
