//! `generate_report` — execution report tool.

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

/// Arguments for `generate_report`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateReportParams {
    /// Absolute path of the repository the report covers.
    #[schemars(description = "Absolute path of the repository the report covers")]
    pub root: String,
    /// Whether analytics are enabled.
    #[schemars(description = "Whether analytics are enabled")]
    pub analytics_enabled: bool,
}

impl InkServer {
    /// Generate an execution report for a repository.
    #[tool(description = "Generate an execution report for a repository")]
    pub async fn generate_report(
        &self,
        Parameters(args): Parameters<GenerateReportParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        notify_progress(&ctx, 0.3, "analyzing for report").await;
        let started = std::time::Instant::now();

        let json = run_analysis(&args.root);
        let (report, error) = match json {
            Ok(document) => {
                let analysis_duration_ms = serde_json::from_str::<serde_json::Value>(&document)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("performance")
                            .and_then(|perf| perf.get("total_duration_ms"))
                            .and_then(serde_json::Value::as_u64)
                    })
                    .unwrap_or(0);
                let mut state = self.state.lock().expect("state lock poisoned");
                let report = state.execution_report(analysis_duration_ms, args.analytics_enabled);
                state.record_run(true, started.elapsed().as_millis() as u64);
                (report, None)
            }
            Err(message) => {
                let mut state = self.state.lock().expect("state lock poisoned");
                state.record_run(false, started.elapsed().as_millis() as u64);
                (
                    state.execution_report(0, args.analytics_enabled),
                    Some(format!("[error] {message}")),
                )
            }
        };

        notify_progress(&ctx, 1.0, "report generated").await;
        let payload = serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string());
        Reporter::from_env().record("generate_report", &payload);
        if let Some(error) = error {
            Ok(CallToolResult::error(vec![ContentBlock::text(error)]))
        } else {
            Ok(CallToolResult::success(vec![ContentBlock::text(payload)]))
        }
    }
}
