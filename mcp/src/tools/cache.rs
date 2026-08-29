//! `get_cache_stats` and `clear_cache` — cache visibility tools.

use rmcp::schemars::JsonSchema;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ErrorData},
    service::RequestContext,
    tool, RoleServer,
};
use serde::Deserialize;

use crate::handler::InkServer;
use crate::reporting::Reporter;

/// Arguments for `get_cache_stats`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CacheParams {
    /// Absolute path of the repository whose cache entries to inspect.
    #[schemars(description = "Absolute path of the repository whose cache entries to inspect")]
    pub root: String,
}

impl InkServer {
    /// Report the cache entries recorded for the repository.
    #[tool(description = "Report the cache entries recorded for the repository")]
    pub async fn get_cache_stats(
        &self,
        Parameters(args): Parameters<CacheParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let state = self.state.lock().expect("state lock poisoned");
        let payload = serde_json::to_string(&state.cache_stats(Some(&args.root)))
            .unwrap_or_else(|_| "{\"entries\":[],\"cacheSizeKb\":0,\"hitRate\":0}".to_string());
        Reporter::from_env().record("get_cache_stats", &payload);
        Ok(CallToolResult::success(vec![ContentBlock::text(payload)]))
    }

    /// Clear the cache entries recorded for the repository.
    #[tool(description = "Clear the cache entries recorded for the repository")]
    pub async fn clear_cache(
        &self,
        Parameters(args): Parameters<CacheParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let _removed = self.mutate_state(|state| state.clear_cache(Some(&args.root)));
        let payload = serde_json::to_string(&ClearCacheResult { cleared: true })
            .unwrap_or_else(|_| "{\"cleared\":true}".to_string());
        Reporter::from_env().record("clear_cache", &payload);
        Ok(CallToolResult::success(vec![ContentBlock::text(payload)]))
    }
}

/// Result document for `clear_cache`.
#[derive(serde::Serialize)]
struct ClearCacheResult {
    cleared: bool,
}
