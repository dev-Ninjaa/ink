//! Human-readable Markdown reports for optimized context.

use crate::models::OptimizedContext;

/// Render a concise Markdown report for an optimized context.
pub fn render_report(context: &OptimizedContext) -> String {
    let metrics = &context.metrics;
    let mut out = String::new();

    out.push_str("# Context Optimization Report\n\n");
    out.push_str(&format!("## Request\n\n- Query: `{}`\n", context.query));
    out.push_str(&format!(
        "- Token budget: {}\n",
        context
            .tokens
            .budget
            .map(|budget| budget.to_string())
            .unwrap_or_else(|| "none".to_owned())
    ));
    out.push_str("\n## Result\n\n");
    out.push_str(&format!(
        "- Files considered: {}\n",
        metrics.files_considered
    ));
    out.push_str(&format!("- Files selected: {}\n", metrics.files_selected));
    out.push_str(&format!(
        "- Files dropped (duplicates): {}\n",
        metrics.files_dropped_duplicates
    ));
    out.push_str(&format!(
        "- Files dropped (non-text): {}\n",
        metrics.files_dropped_non_text
    ));
    out.push_str(&format!(
        "- Files dropped (generated): {}\n",
        metrics.files_dropped_generated
    ));
    out.push_str(&format!(
        "- Files dropped (low relevance): {}\n",
        metrics.files_dropped_low_relevance
    ));
    out.push_str(&format!(
        "- Files dropped (budget): {}\n",
        metrics.files_dropped_budget
    ));
    out.push_str(&format!("- Files excluded: {}\n", metrics.files_excluded));
    out.push_str(&format!(
        "- Tokens before: {}\n",
        context.tokens.tokens_before
    ));
    out.push_str(&format!(
        "- Tokens after: {} ({}% reduction)\n",
        context.tokens.tokens_after, metrics.token_reduction_percent
    ));
    out.push_str(&format!(
        "- Redundancy ratio: {:.4}\n",
        metrics.redundancy_ratio
    ));
    out.push_str(&format!(
        "- Within token budget: {}\n",
        context.tokens.within_budget
    ));
    out.push_str(&format!("- Duration: {:.2} ms\n", metrics.duration_ms));

    if !context.selected.is_empty() {
        out.push_str("\n## Selected Files\n\n");
        for file in &context.selected {
            out.push_str(&format!(
                "- `{}` — {} tokens, relevance {:.2}, score {:.2}\n",
                file.path, file.tokens, file.relevance, file.score
            ));
            for reason in &file.reasons {
                out.push_str(&format!("    - {reason}\n"));
            }
        }
    }

    if !context.dropped.is_empty() {
        out.push_str("\n## Dropped Files\n\n");
        for entry in &context.dropped {
            out.push_str(&format!(
                "- `{}` — {}: {}\n",
                entry.path,
                format_reason(entry.reason),
                entry.detail
            ));
        }
    }

    if !context.warnings.is_empty() {
        out.push_str("\n## Warnings\n\n");
        for warning in &context.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }

    out
}

fn format_reason(reason: crate::models::DroppedReason) -> &'static str {
    match reason {
        crate::models::DroppedReason::Duplicate => "duplicate",
        crate::models::DroppedReason::BudgetExceeded => "budget exceeded",
        crate::models::DroppedReason::NonText => "non-text",
        crate::models::DroppedReason::Generated => "generated",
        crate::models::DroppedReason::LowRelevance => "below relevance threshold",
    }
}
