//! `optimize_context` — optimized context bundle JSON for a query.

use context_optimizer::ContextRequest;
use rmcp::schemars::JsonSchema;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ErrorData},
    service::RequestContext,
    tool, RoleServer,
};
use serde::Deserialize;

use crate::engine::{notify_progress, run_optimize};
use crate::handler::InkServer;
use crate::reporting::Reporter;

/// Arguments for `optimize_context`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct OptimizeParams {
    /// Absolute path of the repository to analyze.
    #[schemars(description = "Absolute path of the repository to analyze")]
    pub root: String,
    /// Developer task or question the context is being prepared for.
    #[schemars(description = "Developer task or question the context is prepared for")]
    pub query: String,
    /// Maximum total bundle size in tokens.
    #[schemars(description = "Maximum total bundle size in tokens")]
    pub max_tokens: Option<usize>,
    /// Minimum normalized relevance (0.0..=1.0) for a file to be selected.
    #[schemars(description = "Minimum normalized relevance (0.0..=1.0) for selection")]
    pub min_relevance: Option<f64>,
}

impl InkServer {
    /// Optimize repository context for a developer query.
    #[tool(
        description = "Optimize repository context for a query and return the optimized context JSON document"
    )]
    pub async fn optimize_context(
        &self,
        Parameters(args): Parameters<OptimizeParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let request = ContextRequest {
            query: args.query,
            max_tokens: args.max_tokens,
            min_relevance: args.min_relevance,
            ..Default::default()
        };
        notify_progress(&ctx, 0.1, "analyzing repository").await;
        notify_progress(&ctx, 0.5, "building dependency graph").await;
        notify_progress(&ctx, 0.8, "optimizing context").await;
        let result = run_optimize(&args.root, &request);
        notify_progress(&ctx, 1.0, "optimization complete").await;
        match result {
            Ok(json) => {
                Reporter::from_env().record("optimize_context", &json);
                Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
            }
            Err(message) => {
                let report = format!("[error] {message}");
                Reporter::from_env().record("optimize_context", &report);
                Ok(CallToolResult::error(vec![ContentBlock::text(report)]))
            }
        }
    }
}
