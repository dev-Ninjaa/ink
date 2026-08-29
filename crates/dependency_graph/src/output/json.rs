//! JSON serialization for dependency graph output.

use crate::models::AnalysisResult;

/// Serialize graph analysis to deterministic pretty JSON.
pub fn to_json(result: &AnalysisResult) -> serde_json::Result<String> {
    serde_json::to_string_pretty(result)
}

/// Serialize graph analysis to compact deterministic JSON.
pub fn to_compact_json(result: &AnalysisResult) -> serde_json::Result<String> {
    serde_json::to_string(result)
}
