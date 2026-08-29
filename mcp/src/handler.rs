//! The MCP server handler: `InkServer` type and its `ServerHandler` impl.
//!
//! `InkServer` is the unit struct that owns the MCP surface. Its tools live in
//! `crate::tools` (router: [`crate::tools::tool_router`]), its prompts in
//! `crate::prompts` (router: [`crate::prompts::prompt_router`]), and its
//! resources in `crate::resources`. This module defines the type, advertises
//! capabilities via `get_info`, implements the resource endpoints, and wires
//! the tool/prompt routers into the `ServerHandler` trait via
//! `#[tool_handler]` / `#[prompt_handler]`.

use rmcp::{
    model::{
        ErrorData, Implementation, ListResourceTemplatesResult, ListResourcesResult,
        PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult, ServerCapabilities,
        ServerInfo,
    },
    prompt_handler,
    service::RequestContext,
    tool_handler, RoleServer, ServerHandler,
};

/// The Ink MCP server.
///
/// Holds orchestration state (agents, cache, run history) shared across tool
/// calls via `Arc<Mutex<_>>`. When `INK_STATE_DIR` is set, mutations persist
/// to `<dir>/ink-state.json` and reload on startup.
#[derive(Clone)]
pub struct InkServer {
    /// Shared orchestration state.
    pub state: crate::state::SharedState,
    /// Persistence directory when `INK_STATE_DIR` is configured.
    state_dir: Option<std::path::PathBuf>,
}

impl InkServer {
    /// Create a server, reloading persisted state when configured.
    pub fn new() -> Self {
        let state = crate::state::RuntimeState::shared();
        let state_dir = crate::state::state_dir_from_env();
        if let Some(dir) = &state_dir {
            *state.lock().expect("state lock poisoned") =
                crate::state::RuntimeState::load_from_dir(dir);
        }
        Self { state, state_dir }
    }

    /// Apply a mutation to the shared state, persisting it when a
    /// `INK_STATE_DIR` is configured. Read-only accessors can lock `state`
    /// directly.
    pub(crate) fn mutate_state<T>(
        &self,
        mutate: impl FnOnce(&mut crate::state::RuntimeState) -> T,
    ) -> T {
        let mut state = self.state.lock().expect("state lock poisoned");
        let result = mutate(&mut state);
        if let Some(dir) = &self.state_dir {
            state.save_to_dir(dir);
        }
        result
    }
}

impl Default for InkServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(router = crate::tools::tool_router())]
#[prompt_handler(router = crate::prompts::prompt_router())]
impl ServerHandler for InkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new("ink", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "Ink gives you structured repository intelligence over MCP.\n\
             Workflow: start every unfamiliar-repo task with analyze_repository.\n\
             Before cross-file edits call build_dependency_graph — cycles and\n\
             central files are your blast radius. When the context window is\n\
             tight, call optimize_context with the task query instead of reading\n\
             files blindly. Use schedule_agents to split entry-point work for\n\
             parallel execution, get_cache_stats before re-analyzing an\n\
             unchanged root, and generate_report to close out. Full guide:\n\
             docs/skills/ink-orchestration/SKILL.md.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(crate::resources::list_resources())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(crate::resources::list_resource_templates())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        crate::resources::read_resource(&request.uri, &context).await
    }
}
