//! `schedule_agents` and `list_agents` — orchestration agent tools.

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
use crate::state::AgentSummaryJson;

/// Arguments for `schedule_agents`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScheduleAgentsParams {
    /// Absolute path of the repository the agents will operate on.
    #[schemars(description = "Absolute path of the repository the agents will operate on")]
    pub root: String,
    /// Maximum number of agents to schedule.
    #[schemars(description = "Maximum number of agents to schedule")]
    pub max_agents: usize,
    /// Whether parallel agent execution is enabled.
    #[schemars(description = "Whether parallel agent execution is enabled")]
    pub parallelism_enabled: bool,
}

impl InkServer {
    /// Schedule one agent per entry point (up to `max_agents`) for a repository.
    #[tool(description = "Schedule orchestration agents (one per entry point) for a repository")]
    pub async fn schedule_agents(
        &self,
        Parameters(args): Parameters<ScheduleAgentsParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        notify_progress(&ctx, 0.2, "analyzing entry points").await;
        let started = std::time::Instant::now();

        let json = run_analysis(&args.root);
        let mut tasks = Vec::new();
        match &json {
            Ok(document) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(document) {
                    if let Some(entries) = value
                        .get("entry_points")
                        .and_then(serde_json::Value::as_array)
                    {
                        for entry in entries {
                            if let Some(path) =
                                entry.get("path").and_then(serde_json::Value::as_str)
                            {
                                tasks.push(path.to_string());
                            }
                        }
                    }
                }
            }
            Err(message) => {
                let report = format!("[error] {message}");
                Reporter::from_env().record("schedule_agents", &report);
                return Ok(CallToolResult::error(vec![ContentBlock::text(report)]));
            }
        }

        let duration_ms = started.elapsed().as_millis() as u64;
        let summary = self.mutate_state(|state| {
            let summary =
                state.schedule_agents(tasks, args.max_agents.max(1), args.parallelism_enabled);
            state.record_run(true, duration_ms);
            summary
        });

        notify_progress(&ctx, 1.0, "agents scheduled").await;
        let payload = serde_json::to_string(&ScheduleAgentsResult {
            accepted: true,
            agents: summary,
        })
        .unwrap_or_else(|_| "{\"accepted\":true}".to_string());
        Reporter::from_env().record("schedule_agents", &payload);
        Ok(CallToolResult::success(vec![ContentBlock::text(payload)]))
    }

    /// List the agents currently scheduled in this server process.
    #[tool(description = "List the agents currently scheduled in this server process")]
    pub async fn list_agents(
        &self,
        _parameters: Parameters<EmptyParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let state = self.state.lock().expect("state lock poisoned");
        let payload = serde_json::to_string(&state.agent_summary())
            .unwrap_or_else(|_| "{\"active\":[],\"completed\":[],\"pending\":[]}".to_string());
        Reporter::from_env().record("list_agents", &payload);
        Ok(CallToolResult::success(vec![ContentBlock::text(payload)]))
    }

    /// Advance agent(s) along their progress toward completion.
    #[tool(
        description = "Advance active agents by a progress step (default 25%); agents reaching 100% complete and pending work is promoted. Pass agent_id to advance one specific agent."
    )]
    pub async fn advance_agents(
        &self,
        Parameters(args): Parameters<AdvanceAgentsParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self.mutate_state(|state| {
            state.advance_agents(args.agent_id.as_deref(), args.step.unwrap_or(25))
        });
        let payload =
            serde_json::to_string(&result).unwrap_or_else(|_| "{\"advancedCount\":0}".to_string());
        Reporter::from_env().record("advance_agents", &payload);
        Ok(CallToolResult::success(vec![ContentBlock::text(payload)]))
    }
}

/// Arguments for `advance_agents`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AdvanceAgentsParams {
    /// Advance only this agent id (defaults to every active agent).
    #[schemars(description = "Advance only this agent id (defaults to every active agent)")]
    pub agent_id: Option<String>,
    /// Percent progress added by this call (default 25).
    #[schemars(description = "Percent progress added by this call (default 25)")]
    pub step: Option<u64>,
}

/// Arguments for `list_agents` (none required).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmptyParams {}

/// Result document for `schedule_agents`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleAgentsResult {
    accepted: bool,
    agents: AgentSummaryJson,
}
