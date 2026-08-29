//! Serialize an [`OptimizedContext`] to JSON.
//!
//! All collections inside the context are deterministically ordered, so the
//! produced JSON is byte-for-byte stable across runs on the same repository
//! and request.

use serde_json::Value;

use crate::error::{Error, Result};
use crate::models::OptimizedContext;

/// Pretty-printed JSON document of an optimized context.
pub fn to_json(context: &OptimizedContext) -> Result<String> {
    serde_json::to_string_pretty(context).map_err(Error::from)
}

/// Compact (single-line) JSON document of an optimized context.
pub fn to_json_compact(context: &OptimizedContext) -> Result<String> {
    serde_json::to_string(context).map_err(Error::from)
}

/// Convert an optimized context to a generic [`Value`] for embedding in
/// larger documents (e.g. the MCP server or extension payloads).
pub fn to_value(context: &OptimizedContext) -> Result<Value> {
    serde_json::to_value(context).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        DedupGroup, DedupSummary, DroppedFile, DroppedReason, FileContext, OptimizationMetrics,
        TokenSummary,
    };

    fn sample() -> OptimizedContext {
        OptimizedContext {
            root: "/tmp/repo".to_owned(),
            optimizer_version: "test".to_owned(),
            query: "auth".to_owned(),
            selected: vec![FileContext {
                path: "src/auth.rs".to_owned(),
                language: Some("rust".to_owned()),
                size_bytes: 100,
                tokens: 25,
                score: 3.0,
                relevance: 1.0,
                reasons: vec!["path token match: `auth`".to_owned()],
                content: Some("fn authenticate() {}".to_owned()),
            }],
            dropped: vec![DroppedFile {
                path: "src/auth_copy.rs".to_owned(),
                reason: DroppedReason::Duplicate,
                detail: "near-duplicate of `src/auth.rs`".to_owned(),
            }],
            dedup: DedupSummary {
                groups: vec![DedupGroup {
                    representative: "src/auth.rs".to_owned(),
                    members: vec!["src/auth_copy.rs".to_owned()],
                    max_similarity: 0.95,
                }],
                files_collapsed: 1,
                bytes_saved: 90,
            },
            tokens: TokenSummary {
                tokens_before: 200,
                tokens_after: 25,
                budget: Some(100),
                within_budget: true,
            },
            metrics: OptimizationMetrics {
                files_considered: 2,
                files_selected: 1,
                files_dropped_budget: 0,
                files_dropped_duplicates: 1,
                files_dropped_non_text: 0,
                files_dropped_generated: 0,
                files_dropped_low_relevance: 0,
                files_excluded: 0,
                bytes_before: 200,
                bytes_after: 100,
                tokens_before: 200,
                tokens_after: 25,
                token_reduction_percent: 87.5,
                redundancy_ratio: 0.875,
                duration_ms: 1.0,
            },
            warnings: Vec::new(),
        }
    }

    #[test]
    fn json_is_deterministic_and_round_trips() {
        let context = sample();
        let first = to_json(&context).unwrap();
        let second = to_json(&context).unwrap();
        assert_eq!(first, second);

        let parsed: OptimizedContext = serde_json::from_str(&first).unwrap();
        assert_eq!(parsed.selected[0].path, "src/auth.rs");
        assert_eq!(parsed.metrics.token_reduction_percent, 87.5);
    }

    #[test]
    fn value_embeds_in_larger_documents() {
        let value = to_value(&sample()).unwrap();
        assert!(value.get("query").is_some());
        assert!(value.get("selected").and_then(|s| s.as_array()).is_some());
    }
}
