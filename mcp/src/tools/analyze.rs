//! `analyze_repository` — Repository Intelligence JSON for a repository.

use rmcp::schemars::JsonSchema;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ErrorData},
    service::RequestContext,
    tool, RoleServer,
};
use serde::Deserialize;

use crate::engine::{notify_progress, run_analysis};
use crate::handler::InkServer;
use crate::reporting::Reporter;

/// Arguments for `analyze_repository`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// Absolute path of the repository to analyze.
    #[schemars(description = "Absolute path of the repository to analyze")]
    pub root: String,
}

impl InkServer {
    /// Analyze a repository with Repository Intelligence.
    #[tool(
        description = "Analyze a repository and return the Repository Intelligence JSON document"
    )]
    pub async fn analyze_repository(
        &self,
        Parameters(args): Parameters<AnalyzeParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        notify_progress(&ctx, 0.1, "analyzing repository").await;
        let started = std::time::Instant::now();
        let result = run_analysis(&args.root);
        notify_progress(&ctx, 1.0, "analysis complete").await;
        match result {
            Ok(json) => {
                let bytes = json.len() as u64;
                self.mutate_state(|state| {
                    state.record_cache_hit(&args.root, "repository analysis", bytes / 1024);
                    state.record_run(true, started.elapsed().as_millis() as u64);
                });
                Reporter::from_env().record("analyze_repository", &json);
                Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
            }
            Err(message) => {
                let report = format!("[error] {message}");
                self.mutate_state(|state| {
                    state.record_run(false, started.elapsed().as_millis() as u64)
                });
                Reporter::from_env().record("analyze_repository", &report);
                Ok(CallToolResult::error(vec![ContentBlock::text(report)]))
            }
        }
    }
}
