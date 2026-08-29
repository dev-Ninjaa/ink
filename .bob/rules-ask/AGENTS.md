# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Non-Obvious Documentation Context

- **`extension/` is entirely a human observability cockpit**, not the AI agent consumer. Agents (IBM Bob, Claude Code, Cursor) connect directly to `ink_mcp` over stdio/HTTP and never touch the VS Code extension.
- **`mcp/src/lib.rs` exists only for integration tests** — the binary entry is `main.rs`. The lib just re-exports `InkServer` so `mcp/tests/integration_tests.rs` can import without going through main.
- **`extension/src/mocks/`** contains `MockRuntime`, `MockRuntimeFactory`, and `mockData.ts` — these are used in tests only. The production path is always `McpRuntime` → `SdkMcpClient` → real `ink_mcp` process.
- **`contracts/index.ts` is the single import point** for all request/response types. Do not import from `McpContracts.ts` or `RuntimeContracts.ts` directly.
- **`mcpMapping.ts` is the sole translation layer** between the MCP server's JSON (snake_case for engine tools, camelCase for orchestration tools) and the TypeScript model types. All new server document shapes belong here.
- **`docs/skills/ink-orchestration/SKILL.md`** is the agent skill file referenced by MCP `initialize` instructions — it is served as-is to agents; edits to it immediately affect agent behavior.
- **`INK_STATE_DIR` persistence**: the state file is `<dir>/ink-state.json`, written atomically (tmp + rename). The format is defined by the `RuntimeState` struct in `mcp/src/state.rs`.
- **`INK_REPORT_DIR` reporting**: writes `<tool>-<timestamp>.json` + appends to `report.md`. Only active when env var is set — stdio is clean otherwise.
- **HTTP transport is NOT compiled in by default** — requires `--features http`. The Docker image and compose stack enable it; the standalone binary and npx install do not.
