# Plan: Make Ink MCP Server proper + remotely hostable

Status: **Implemented & verified (all 4 parts) + modular refactor**
Target: `ink.mcpServer` (rmcp 2.2.0, Rust)

## Module layout (post-refactor)

```
src/
├── main.rs       # entry point + transport setup (stdio/http CLI) + auth middleware
├── lib.rs        # library facade (for integration tests)
├── handler.rs    # InkServer type + ServerHandler impl (get_info, resources) + tool/prompt macros
├── engine.rs     # engine bridge (run_analysis/run_graph/run_optimize) + notify_progress
├── reporting.rs  # Reporter (file-based, INK_REPORT_DIR)
├── state.rs      # process-local orchestration state (agents, cache, run history)
├── tools/        # one file per tool + assembled ToolRouter
│   ├── analyze.rs   # analyze_repository
│   ├── graph.rs     # build_dependency_graph
│   ├── optimize.rs  # optimize_context
│   ├── agents.rs    # schedule_agents / list_agents
│   ├── cache.rs     # get_cache_stats / clear_cache
│   └── report.rs    # generate_report
├── prompts/      # one file per prompt + assembled PromptRouter
│   └── orchestrate.rs
└── resources/    # resource helpers (list/templates/read)
tests/
└── integration_tests.rs  # E2E: spawns the real binary, drives raw JSON-RPC over stdio
```

Notes:
- `#[tool]` / `#[prompt]` macros emit `<name>_tool_attr()` / `<name>_prompt_attr()`
  metadata fns; routers are assembled manually in `tools/mod.rs` /
  `prompts/mod.rs` via `ToolRouter::new().with_route((...))` because
  `#[tool_router]` only collects same-block items.
- `#[tool_handler]` / `#[prompt_handler]` use
  `router = crate::tools::tool_router()` / `router = crate::prompts::prompt_router()`.
- `JsonSchema` derives resolve through `rmcp::schemars` (1.x); the direct
  `schemars` dep was dropped.

## Current state (verified)

- `ink.mcpServer` — rmcp 2.2.0, stdio-only, 3 tools returning plain `String`;
  errors are `"[error] ..."` text prefixes (no `isError` signal); no
  Resources, Prompts, progress notifications, or capability advertisement
  beyond tools.
- rmcp 2.2 already ships everything needed:
  - `StreamableHttpService` (a `tower` service),
  - `StreamableHttpServerConfig` (SSE keep-alive, allowed hosts/origins,
    stateful/stateless mode, session store),
  - session managers: `LocalSessionManager` (in-memory) /
    `NeverSessionManager` (stateless),
  - `auth` feature (OAuth2/DCR via the `oauth2` crate),
  - `ContentBlock::{Text, Image, Audio, Resource, ResourceLink}` and
    `CallToolResult.is_error`.
- `InkServer` already implements `ServerHandler`, and rmcp provides
  `impl<H: ServerHandler> Service<RoleServer> for H` — so the existing
  handler plugs straight into the HTTP service factory.

## Part 1 — Harden the tools (structured, spec-compliant)

**`src/main.rs`** — change each tool handler from `-> String` to
`-> Result<CallToolResult, ErrorData>` (rmcp `#[tool]` macro supports this):

- Success: `CallToolResult::success(vec![ContentBlock::text(json)])`.
- Tool-level failure: `CallToolResult::error(vec![ContentBlock::text("[error] ...")])`
  → MCP responds with `isError: true` (message visible to caller);
  `Err(ErrorData)` is reserved for protocol/infra errors (per rmcp docs).
- Keep `Reporter::record(...)` in all three handlers; fire on both the success
  and the error path so failures are still persisted to `report.md`.
- Params already derive `JsonSchema` via `schemars` — keep as-is.

**Progress** (long jobs: full repo scan + graph + optimize):

- `RequestContext<RoleServer>` is a valid second tool-handler param
  (implements `FromContextPart`). Helper `notify_progress(ctx, progress, msg)`
  reads `ctx.meta.get_progress_token()` and calls
  `ctx.peer.notify_progress(ProgressNotificationParam::new(token, p).with_message(msg))`.

## Part 2 — Resources + Prompts (make it a full MCP server)

- **Resources**: `ink://analysis/{root}` and `ink://graph/{root}` as read-only
  MCP resource templates (no `#[resource]` macro exists; implemented manually
  via `list_resource_templates` + `read_resource`).
- **Prompts**: `orchestrate_agent` prompt via `#[prompt_router]` +
  `#[prompt_handler]` (auto-generates prompt arguments from `Parameters<T>`).
- **Capabilities**: manual `get_info` override advertises
  `tools + resources + prompts` via `ServerCapabilities::builder()`.
- `#[tool_handler]` + `#[prompt_handler]` coexist on one `impl ServerHandler`
  block; both skip auto-generating `get_info` when it is present.

## Part 3 — Streamable HTTP transport + auth (remote hosting)

**`src/main.rs`**:

- CLI/env: `--transport stdio|http` (default `stdio`), `--addr 0.0.0.0:3000`.
- For `http`: build `StreamableHttpService` with a service factory
  `|| Ok(InkServer)`, `LocalSessionManager::default()`, and
  `StreamableHttpServerConfig` (loopback hosts by default;
  `with_allowed_origins` from env for browser clients).
- Mount via axum `Router::new().route_service("/mcp", service)` (a tower
  Service, so `route_service`, not `route`) at `/mcp`, behind a
  `bearer_token_auth` middleware.
- Keep `INK_REPORT_DIR` reporting working in both transports — it already
  writes to files, not stdout, so the stdio protocol stays clean.

**Auth** — decision (b): bearer-token middleware. When `INK_API_TOKEN` is set,
requests without `Authorization: Bearer <token>` get 401; unset/empty disables
auth. `INK_ALLOWED_HOSTS` / `INK_ALLOWED_ORIGINS` gate Host/Origin.

## Part 4 — Config, docs, testing

- `[features] http = ["dep:axum", "rmcp/transport-streamable-http-server"]`;
  default build stays stdio-only, HTTP builds with `cargo build --features http`.
- `Cargo.toml` deps: optional `axum 0.8` (`features = ["http1","tokio"]`,
  `default-features = false`).
- `README.md`: HTTP launch command, `curl` smoke tests, auth header usage.
- Test:
  - Existing 6 reporting unit tests stay green (`cargo test --features http`).
  - New E2E `tests/integration_tests.rs` (7 tests): spawns the real binary over
    stdio, drives raw JSON-RPC — initialize capabilities, `tools/list`,
    `tools/call` isError, `resources/templates/list`, `resources/read` on a real
    repo, unknown-URI rejection, `prompts/list`.
  - Live `curl` over HTTP: initialize + `tools/list` + `tools/call` +
    `resources/*` + `prompts/get`, and 401 checks for missing/wrong token.
  - stdio end-to-end smoke test re-verified after the rewrite.

## Decisions confirmed

1. **Auth**: (b) bearer-token middleware via `INK_API_TOKEN`.
2. **Progress**: included in Part 1.
3. **HTTP endpoint path**: `/mcp`.

## Proposed execution order

1. Part 1 (structured tool results + error signaling).
2. Part 2 (Resources, Prompts, capabilities).
3. Part 3 (HTTP transport + auth).
4. Part 4 (features, docs, tests).