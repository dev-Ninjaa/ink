//! Engine bridge: wraps the Ink engine crates and shared MCP helpers.
//!
//! Thin, error-string-mapped wrappers over `repository_intelligence`,
//! `dependency_graph`, and `context_optimizer`, plus the progress-notification
//! helper used by tools and resources. Keeps the MCP layer free of engine
//! details.

use context_optimizer::{optimize_context, ContextRequest};
use dependency_graph::{analyze_dependencies, output::json as graph_json};
use repository_intelligence::{analyze_repository, output::json as analysis_json};
use rmcp::{model::ProgressNotificationParam, service::RequestContext, RoleServer};

/// Run Repository Intelligence and return its JSON document.
pub fn run_analysis(root: &str) -> Result<String, String> {
    let analysis = analyze_repository(root).map_err(|error| format!("analysis failed: {error}"))?;
    analysis_json::to_json(&analysis).map_err(|error| format!("serialization failed: {error}"))
}

/// Run Repository Intelligence + dependency graph and return the graph JSON.
pub fn run_graph(root: &str) -> Result<String, String> {
    let analysis = analyze_repository(root).map_err(|error| format!("analysis failed: {error}"))?;
    let graph = analyze_dependencies(&analysis);
    graph_json::to_json(&graph).map_err(|error| format!("serialization failed: {error}"))
}

/// Run the full context-optimization pipeline and return its JSON document.
pub fn run_optimize(root: &str, request: &ContextRequest) -> Result<String, String> {
    let analysis = analyze_repository(root).map_err(|error| format!("analysis failed: {error}"))?;
    let graph = analyze_dependencies(&analysis);
    let context = optimize_context(&analysis, Some(&graph), request)
        .map_err(|error| format!("optimization failed: {error}"))?;
    context_optimizer::output::json::to_json(&context)
        .map_err(|error| format!("serialization failed: {error}"))
}

/// Send a progress notification when the request carried a progress token.
pub async fn notify_progress(ctx: &RequestContext<RoleServer>, progress: f64, message: &str) {
    let Some(token) = ctx.meta.get_progress_token() else {
        return;
    };
    let param = ProgressNotificationParam::new(token, progress).with_message(message);
    let _ = ctx.peer.notify_progress(param).await;
}
