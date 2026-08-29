//! Prompt templates: one file per prompt, plus the assembled router.
//!
//! Each `#[prompt]` handler lives in its own file with its arguments struct.
//! The `#[prompt]` macro emits a `pub fn <name>_prompt_attr()` metadata
//! function next to each handler; `prompt_router()` assembles them into a
//! single [`PromptRouter`].

pub mod orchestrate;

use rmcp::handler::server::router::prompt::PromptRouter;

use crate::handler::InkServer;

/// Assemble the router that registers every Ink prompt.
pub fn prompt_router() -> PromptRouter<InkServer> {
    PromptRouter::new().with_route((
        InkServer::orchestrate_agent_prompt_attr(),
        InkServer::orchestrate_agent,
    ))
}
