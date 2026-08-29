# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Critical Coding Rules

- **New MCP tool**: add `impl InkServer` in `mcp/src/tools/<name>.rs`, add `pub mod <name>` in `tools/mod.rs`, register `.with_route((InkServer::<name>_tool_attr(), InkServer::<name>))` in `tool_router()`. The `#[tool]` macro generates the `_tool_attr()` function.
- **State mutations**: always use `self.mutate_state(|state| { ... })` — never lock `self.state` for writes. Read-only accesses may lock directly with `self.state.lock()`.
- **Tool errors**: return `Ok(CallToolResult::error(vec![ContentBlock::text(msg)]))`, never `Err(ErrorData)`. The distinction is between a tool-level error (`isError: true`) vs a protocol error.
- **New Runtime operation**: must be added to `Runtime` (interface), `McpRuntime` (real impl), `MockRuntime` (test impl), and `MockRuntimeFactory` — all four, together.
- **All webview user data**: must go through `escapeHtml()` from `extension/src/utils/webview.ts`. Adding scripts to Dashboard/Analytics views requires changing `enableScripts` to `true` AND adding `script-src 'nonce-${nonce}'` to the CSP in the HTML template.
- **TS error handling**: wrap with `toInkError(error, "Category")` from `errors/InkError.ts` before logging. Use `void` prefix for fire-and-forget VS Code API calls (`void vscode.window.showErrorMessage(...)`).
- **`noUncheckedIndexedAccess` is on**: array index access returns `T | undefined`; always guard.
- **Extension test ID**: `vscode.extensions.getExtension("ink.ink")` — publisher is `"ink"`, name is `"ink"`.
- **Do not call `run_analysis`/`run_graph`/`run_optimize` inside `tokio::spawn_blocking`** — they parallelise with Rayon themselves and should be called directly from async tool handlers.
- **camelCase boundary**: `mcp/src/state.rs` orchestration structs use `#[serde(rename_all = "camelCase")]`; engine crate output is `snake_case`. Keep this split — do not add camelCase to engine output structs.
