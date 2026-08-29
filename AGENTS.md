# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Repository Layout

Four Rust workspace members (`crates/repository_intelligence`, `crates/dependency_graph`, `crates/context_optimizer`, `mcp`) plus a TypeScript VS Code extension (`extension/`) and a Node.js npm wrapper (`npm/ink-mcp/`). All Rust commands must be run from the workspace root; extension commands must be run from `extension/`.

## Build / Test Commands

```bash
# Rust — run from workspace root
cargo test --workspace                          # all unit + integration tests
cargo test -p ink_mcp                           # MCP server only (includes E2E raw JSON-RPC tests)
cargo test -p repository_intelligence           # single crate
cargo test <test_name>                          # single test by name
cargo test -p ink_mcp -- orchestration_tools    # single integration test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --features http -p ink_mcp          # HTTP transport (optional feature, not default)

# Extension — run from extension/
npm ci
npm run compile                                 # tsc -p ./ → out/
npm test                                        # compile + headless VS Code (xvfb-run on Linux)
```

**There is no single-test command for the extension** — `npm test` runs the whole Mocha suite via `@vscode/test-electron` against a pinned VS Code 1.92.0 instance. Individual tests cannot be run in isolation without modifying `runTest.ts`.

## Critical Non-Obvious Patterns

### Rust / MCP Server

- **New tool = four files minimum**: add `impl InkServer` block in `mcp/src/tools/<name>.rs`, register in `mcp/src/tools/mod.rs::tool_router()`, add `pub mod` in `tools/mod.rs`. The `#[tool]` macro auto-generates `<name>_tool_attr()` — this generated fn is what `tool_router()` calls.
- **State mutations must go through `InkServer::mutate_state()`**, not by locking `self.state` directly — it handles optional `INK_STATE_DIR` persistence automatically. Read-only accesses may lock directly.
- **All tool errors return `CallToolResult::error(...)` not `Err(ErrorData)`** — failing at the JSON-RPC level is wrong; use `isError: true` content so the MCP client's `isError` field is set.
- **Engine crates are called synchronously** (`run_analysis`, `run_graph`, `run_optimize` in `engine.rs`). They block the Tokio thread — do not call them inside `spawn_blocking`; they use Rayon internally.
- **Server JSON uses `snake_case` for engine outputs, `camelCase` for orchestration tool outputs** (`#[serde(rename_all = "camelCase")]` on all state JSON structs in `state.rs`).
- **HTTP transport is behind the `http` cargo feature** — it must be enabled explicitly (`--features http`). The default build produces a stdio-only binary.
- **`mcp/src/lib.rs` exists only for integration testing** — the binary uses `main.rs`; the lib exposes `InkServer` so `mcp/tests/integration_tests.rs` can import it without re-exporting via `main.rs`.

### TypeScript Extension

- **Extension ID in tests is `"ink.ink"`** (publisher.name = `"ink"."ink"` from `package.json`) — if the publisher changes, the test `vscode.extensions.getExtension("ink.ink")` breaks.
- **tsconfig has `noUncheckedIndexedAccess: true`** — array/map reads return `T | undefined`; always null-check index results.
- **All webview HTML strings must use `escapeHtml()` from `extension/src/utils/webview.ts`** for any user-visible data. Static strings do not need escaping.
- **Dashboard/Analytics webviews have `enableScripts: false`**; Dependency Graph panel has `enableScripts: true` (requires inline `<script nonce="...">` and a `script-src 'nonce-...'` CSP header). Adding scripts to dashboard/analytics webviews requires changing both `enableScripts` and the CSP.
- **`Runtime` interface** (`extension/src/services/Runtime.ts`) is the contract between `McpRuntime` (real) and `MockRuntime` (tests). New operations must be added to `Runtime`, `McpRuntime`, `MockRuntime`, and `MockRuntimeFactory` together.
- **`contracts/index.ts` re-exports everything** from `McpContracts.ts` and `RuntimeContracts.ts` — import from `"../contracts"` not the individual files.
- **`mcpMapping.ts` is the sole snake_case ↔ camelCase boundary** — server document shapes (snake_case) are defined there and converted to TypeScript models. Never assume camelCase from the server for engine tools.
- **Extension tests run inside a real VS Code host** — no mocking of `vscode.*` APIs is possible. Tests in `suite/` use `MockRuntimeFactory` to avoid real MCP connections.

### npm Wrapper

- `postinstall.js` downloads the platform binary from GitHub Releases. The version is read from `npm/ink-mcp/package.json`. Override with `INK_MCP_VERSION` env var to point at a different tag.

## Code Style

- **Rust**: `snake_case` for all identifiers; `PascalCase` for types/traits. Errors use `thiserror`-derived enums, never `anyhow` in library crates (only in `mcp/`). Doc comments on all `pub` items.
- **TypeScript**: `camelCase` for functions/variables, `PascalCase` for classes/interfaces/types. `readonly` on all interface fields. No `any` — use `unknown` and narrow. `void` return for fire-and-forget async calls (prefix with `void` at call site: `void vscode.window.showErrorMessage(...)`).
- **Error handling (TS)**: always wrap unknown errors with `toInkError(error, category)` from `errors/InkError.ts` before logging or showing to users. Never surface raw `Error.message` in UI.
- **Conventional Commits** required for all commit messages.
