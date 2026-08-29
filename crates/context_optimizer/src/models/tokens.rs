//! Token accounting models.

use serde::{Deserialize, Serialize};

/// Token accounting for a context optimization run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TokenSummary {
    /// Approximate tokens of the full candidate set before optimization.
    pub tokens_before: usize,
    /// Approximate tokens of the optimized bundle.
    pub tokens_after: usize,
    /// Requested token budget, when one was given.
    pub budget: Option<usize>,
    /// Whether the optimized bundle fits inside the requested budget.
    pub within_budget: bool,
}
