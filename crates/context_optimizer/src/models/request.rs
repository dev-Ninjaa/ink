//! Request model describing what context should be optimized.

use serde::{Deserialize, Serialize};

/// A request to produce an optimized context bundle.
///
/// `query` is free-form developer intent (e.g. `"fix the auth bug"`). The
/// optional `include_paths` / `exclude_paths` filters are repository-relative
/// paths or path prefixes that narrow or widen the candidate set. `max_tokens`
/// and `max_files` optionally cap the resulting bundle, and `min_relevance`
/// optionally drops files whose normalized relevance falls below a threshold.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContextRequest {
    /// The developer task or question the context is being prepared for.
    #[serde(default)]
    pub query: String,
    /// Repository-relative paths or prefixes that must be considered. When
    /// non-empty, only files matching at least one entry are candidates.
    #[serde(default)]
    pub include_paths: Vec<String>,
    /// Repository-relative paths or prefixes that are never selected.
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    /// Maximum total size of the optimized bundle in tokens.
    #[serde(default)]
    pub max_tokens: Option<usize>,
    /// Maximum number of files to select.
    #[serde(default)]
    pub max_files: Option<usize>,
    /// Minimum normalized relevance (`0.0..=1.0`) for a file to be selected.
    /// Files below this threshold are dropped with a `low_relevance` reason.
    /// Values outside `0.0..=1.0` are clamped.
    #[serde(default)]
    pub min_relevance: Option<f64>,
}
