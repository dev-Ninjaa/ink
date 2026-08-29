---
name: add-ink-tool
description: >-
  Add a new MCP tool to the Ink server (ink_mcp). Use when the task involves
  implementing a new tool that agents or the VS Code extension can call over MCP.
version: 1.0.0
---

# Adding a New Ink MCP Tool

## Checklist (complete in order)

### 1. Rust server — new tool file
Create `mcp/src/tools/<name>.rs` with this structure:
```rust
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::{CallToolResult, ContentBlock, ErrorData}, service::RequestContext, tool, RoleServer};
use serde::Deserialize;
use crate::handler::InkServer;
use crate::reporting::Reporter;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyToolParams {
    #[schemars(description = "...")]
    pub root: String,
}

impl InkServer {
    #[tool(description = "...")]
    pub async fn my_tool(
        &self,
        Parameters(args): Parameters<MyToolParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        // On error: return Ok(CallToolResult::error(...)) — never Err(ErrorData)
        // State mutations: self.mutate_state(|state| { ... })
        // Progress: crate::engine::notify_progress(&ctx, 0.5, "message").await
        // Reporting: Reporter::from_env().record("my_tool", &json);
        todo!()
    }
}
```

### 2. Register in mod.rs
In `mcp/src/tools/mod.rs`:
- Add `pub mod <name>;` at the top
- Add `.with_route((InkServer::my_tool_tool_attr(), InkServer::my_tool))` to `tool_router()`

The `#[tool]` macro auto-generates `my_tool_tool_attr()` — this is not hand-written.

### 3. Extension — if the tool needs a UI surface
- Add request/response types to `extension/src/contracts/RuntimeContracts.ts`
- Add the method to the `Runtime` interface (`extension/src/services/Runtime.ts`)
- Implement it in `McpRuntime` (`extension/src/services/McpRuntime.ts`) using `this.invoke<Doc>("my_tool", { ... })`
- Add a no-op stub to `MockRuntime` (`extension/src/mocks/MockRuntime.ts`)
- Add a `describe()` return value to `MockRuntimeFactory` if version changes
- Add document shape + converter to `extension/src/mcp/mcpMapping.ts`
- Wire a command in `extension/src/commands/registerCommands.ts` + `extension/package.json`

### 4. Integration test
Add a test to `mcp/tests/integration_tests.rs` — spawn the real binary, drive JSON-RPC over stdio, assert `isError: false` and the expected document shape.

### 5. Update docs
Add a row to the Tools table in `README.md` and a row to the decision table in `docs/skills/ink-orchestration/SKILL.md`.

## Key invariants
- `root` parameter: always an absolute path; the engine crates reject relative paths with a clear error
- `snake_case` for engine output serde, `camelCase` for orchestration state serde
- Never call `run_analysis` / `run_graph` / `run_optimize` inside `tokio::spawn_blocking`
- Tools that mutate state must use `self.mutate_state()`, not direct lock
