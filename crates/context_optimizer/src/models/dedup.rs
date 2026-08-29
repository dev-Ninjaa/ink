//! Near-duplicate deduplication models.

use serde::{Deserialize, Serialize};

/// A group of near-duplicate files collapsed into one representative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DedupGroup {
    /// Path of the file kept as the group's representative.
    pub representative: String,
    /// Paths of the files collapsed into the representative.
    pub members: Vec<String>,
    /// Highest pairwise similarity observed against the representative.
    pub max_similarity: f64,
}

/// Summary of near-duplicate removal for the whole run.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DedupSummary {
    /// Duplicate groups, sorted by representative path.
    pub groups: Vec<DedupGroup>,
    /// Total number of files collapsed into representatives.
    pub files_collapsed: usize,
    /// Total bytes of collapsed files that were not sent.
    pub bytes_saved: u64,
}
