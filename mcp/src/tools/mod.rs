//! Tool definitions: one file per tool, plus the assembled router.
//!
//! Each `#[tool]` handler lives in its own file with its `Parameters` struct.
//! The `#[tool]` macro emits a `pub fn <name>_tool_attr()` metadata function
//! next to each handler; `tool_router()` assembles them into a single
//! [`ToolRouter`].

pub mod agents;
pub mod analyze;
pub mod cache;
pub mod graph;
pub mod optimize;
pub mod report;

use rmcp::handler::server::router::tool::ToolRouter;

use crate::handler::InkServer;

/// Assemble the router that registers every Ink tool.
pub fn tool_router() -> ToolRouter<InkServer> {
    ToolRouter::new()
        .with_route((
            InkServer::analyze_repository_tool_attr(),
            InkServer::analyze_repository,
        ))
        .with_route((
            InkServer::build_dependency_graph_tool_attr(),
            InkServer::build_dependency_graph,
        ))
        .with_route((
            InkServer::optimize_context_tool_attr(),
            InkServer::optimize_context,
        ))
        .with_route((
            InkServer::schedule_agents_tool_attr(),
            InkServer::schedule_agents,
        ))
        .with_route((InkServer::list_agents_tool_attr(), InkServer::list_agents))
        .with_route((
            InkServer::advance_agents_tool_attr(),
            InkServer::advance_agents,
        ))
        .with_route((
            InkServer::get_cache_stats_tool_attr(),
            InkServer::get_cache_stats,
        ))
        .with_route((InkServer::clear_cache_tool_attr(), InkServer::clear_cache))
        .with_route((
            InkServer::generate_report_tool_attr(),
            InkServer::generate_report,
        ))
}
