//! `orchestrate_agent` — structured orchestration prompt for a coding agent.

use rmcp::schemars::JsonSchema;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{ErrorData, PromptMessage, Role},
    prompt,
};
use serde::Deserialize;

use crate::handler::InkServer;

/// Arguments for the `orchestrate_agent` prompt.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct OrchestratePromptArgs {
    /// Absolute path of the repository to operate on.
    #[schemars(description = "Absolute path of the repository to operate on")]
    pub root: String,
    /// The developer task the agent should orchestrate.
    #[schemars(description = "The developer task the agent should orchestrate")]
    pub task: String,
    /// Maximum total context bundle size in tokens.
    #[schemars(description = "Maximum total context bundle size in tokens")]
    pub max_tokens: Option<usize>,
}

impl InkServer {
    /// Orchestrate an agent on a repository.
    #[prompt(
        name = "orchestrate_agent",
        description = "Orchestrate a coding agent: analyze the repository, build the dependency graph and assemble an optimized context bundle for the task."
    )]
    pub async fn orchestrate_agent(
        &self,
        Parameters(args): Parameters<OrchestratePromptArgs>,
    ) -> Result<Vec<PromptMessage>, ErrorData> {
        let mut messages = vec![
            PromptMessage::new_text(Role::User, "You are orchestrating a coding agent. Execute the following pipeline against the repository root, in order:"),
            PromptMessage::new_text(Role::User, format!(
                "1. Analyze the repository at `{}` with the `analyze_repository` tool.",
                args.root
            )),
            PromptMessage::new_text(Role::User, "2. Build the dependency graph with the `build_dependency_graph` tool."),
        ];
        if let Some(max_tokens) = args.max_tokens {
            messages.push(PromptMessage::new_text(
                Role::User,
                format!(
                    "3. Optimize context with the `optimize_context` tool (max_tokens = {max_tokens}) for this task: {}",
                    args.task
                ),
            ));
        } else {
            messages.push(PromptMessage::new_text(
                Role::User,
                format!(
                    "3. Optimize context with the `optimize_context` tool for this task: {}",
                    args.task
                ),
            ));
        }
        messages.push(PromptMessage::new_text(
            Role::User,
            "Report the results to the developer in a concise summary, citing the repository root and any limitations you encountered.",
        ));
        Ok(messages)
    }
}
