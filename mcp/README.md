# Ink MCP Server

Model Context Protocol server exposing the Ink orchestration engine —
`repository_intelligence`, `dependency_graph`, and `context_optimizer` — to
agentic IDEs (IBM Bob, Claude Code, Cursor, ...).

Transports: **stdio** (default, for local MCP clients) and **Streamable HTTP**
(remote, with optional bearer-token auth).

## Tools

| Tool | Description |
|------|-------------|
| `analyze_repository` | Repository Intelligence JSON for a repo (files, languages, entry points, modules, relationships). |
| `build_dependency_graph` | Dependency graph JSON for a repo (nodes, edges, cycles, central files, reachability). |
| `optimize_context` | Optimized context bundle JSON for a query (selected files, dropped files, token metrics). |
| `schedule_agents` | Schedule orchestration agents (one per entry point, capped by `max_agents`) for a repo. |
| `list_agents` | List the agents currently scheduled in the server process. |
| `advance_agents` | Advance active agents by a progress step (default 25%); completed agents promote pending work. |
| `get_cache_stats` | Cache entries, total size, and hit rate for a repo (populated by analysis runs). |
| `clear_cache` | Clear the cache entries for a repo. |
| `generate_report` | Execution report with timeline and runtime statistics for a repo. |

The orchestration tools (`schedule_agents`, `list_agents`, `get_cache_stats`,
`clear_cache`, `generate_report`) share process-local state — scheduled
agents, cache entries, and run history — so MCP clients observe live activity
instead of mock data. Set `INK_STATE_DIR` to a directory to persist this
state across server restarts as `<dir>/ink-state.json`; unset, it remains
process-local.

## Resources

| URI template | Description |
|--------------|-------------|
| `ink://analysis/{root}` | Repository Intelligence JSON document for a repository root. |
| `ink://graph/{root}` | Dependency graph JSON document for a repository root. |

`{root}` is substituted with the absolute repository path, e.g.
`ink://analysis//mnt/e/Codebase/Hackathon/ink-ibm/ink/mcp`.

## Prompts

| Prompt | Description |
|--------|-------------|
| `orchestrate_agent` | Pipeline instruction (analyze → graph → optimize) for a given `root` and `task`. |

## Prerequisites

- Rust toolchain (stable, MSRV 1.75) on the same OS as the MCP client.
- The `ink` engine workspace (this crate depends on it via `path` deps):
  `../crates/{repository_intelligence,dependency_graph,context_optimizer}`.

## Build

From this directory:

```powershell
# Windows
cargo build
# binary: target\debug\ink_mcp.exe

# Linux / WSL
cargo build
# binary: target/debug/ink_mcp
```

To include the Streamable HTTP transport:

```powershell
cargo build --features http
```

Note: the binary must be built on the OS where the MCP client runs. A WSL
build produces a Linux ELF that will not run on Windows and vice versa.

## Run Tests

```powershell
cargo test          # unit tests, incl. reporting
cargo build         # confirm zero warnings
```

## Test with MCP Inspector (Windows)

Easiest: double-click `run-mcp-inspector.bat` — it creates the reports dir,
builds the binary if missing, and launches MCP Inspector.

Or manually:

```powershell
set INK_REPORT_DIR=E:\Codebase\Hackathon\ink-ibm\ink\mcp\reports
npx @modelcontextprotocol/inspector
```

In the Inspector connection form:

- Transport: `stdio`
- Command: `E:\Codebase\Hackathon\ink-ibm\ink\mcp\target\debug\ink_mcp.exe`
- Args: *(empty)*
- Environment variables: `INK_REPORT_DIR=E:\Codebase\Hackathon\ink-ibm\ink\mcp\reports`

Then call the three tools with a repository path, e.g.:

- `analyze_repository` → `{"root": "E:\\Codebase\\Hackathon\\ink-ibm\\ink"}`
- `build_dependency_graph` → `{"root": "E:\\Codebase\\Hackathon\\ink-ibm\\ink.extension"}`
- `optimize_context` →
  `{"root": "E:\\Codebase\\Hackathon\\ink-ibm\\ink.extension", "query": "where is the main runtime entry point", "max_tokens": 3000}`

## Get the Reports

When `INK_REPORT_DIR` is set, every tool call writes into that directory:

- `<tool>-<YYYYMMDD-HHMMSS-mmm>.json` — the raw pretty-printed JSON result of
  each call (one file per tool call).
- `report.md` — a single combined human-readable markdown report, appended on
  every call (one `### <tool>` section per call).

Example `report.md`:

```markdown
### analyze_repository — 2026-08-28 15:55:15 UTC

- **root:** E:\Codebase\Hackathon\ink-ibm\ink.extension
- **files:** 46 · **dirs:** 17 · **bytes:** 175124
- **languages:** json=3, markdown=1, typescript=39
- **entry points:** src/extension.ts (0.85)
- **modules:** 10 · **relationships:** 112
```

If `INK_REPORT_DIR` is unset or empty, reporting is disabled (no files written)
and the stdio transport stays clean. Reporting never writes to stdout.

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `INK_REPORT_DIR` | Directory for timestamped JSON + `report.md` output | *(disabled)* |
| `INK_API_TOKEN` | Bearer token required for HTTP requests | *(auth disabled)* |
| `INK_ALLOWED_HOSTS` | Comma-separated `Host` header allow-list | localhost, 127.0.0.1, [::1] (+ bind host when `0.0.0.0`) |
| `INK_ALLOWED_ORIGINS` | Comma-separated `Origin` allow-list | empty (Origin not enforced) |

## Serve over HTTP (remote access)

Build with the `http` feature, then run with `--transport http`:

```powershell
# Windows (remote)
set INK_API_TOKEN=super-secret-token
cargo run --features http -- --transport http --addr 0.0.0.0:3000

# Linux / WSL
INK_API_TOKEN=super-secret-token \
cargo run --features http -- --transport http --addr 0.0.0.0:3000
```

MCP clients connect to `http://<host>:3000/mcp`.

- **Auth:** when `INK_API_TOKEN` is set, every request must include
  `Authorization: Bearer <token>`; otherwise requests are rejected with 401.
  When unset/empty, auth is disabled.
- **Hosts:** when `INK_ALLOWED_HOSTS` is set, only those comma-separated host
  values are accepted in the `Host` header. When unset, localhost + the bind
  host are allowed.
- **Origins:** set `INK_ALLOWED_ORIGINS` to restrict the `Origin` header of
  browser-based clients.

### Smoke test over HTTP

```bash
# 1. initialize (capture the Mcp-Session-Id response header)
curl -i -X POST http://127.0.0.1:3000/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Authorization: Bearer <token>' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"curl","version":"0.1"}}}'

# 2. list tools (reuse the Mcp-Session-Id header value)
curl -X POST http://127.0.0.1:3000/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Mcp-Session-Id: <session-id>' \
  -H 'Authorization: Bearer <token>' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# 3. call a tool
curl -X POST http://127.0.0.1:3000/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Mcp-Session-Id: <session-id>' \
  -H 'Authorization: Bearer <token>' \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"build_dependency_graph","arguments":{"root":"/mnt/e/Codebase/Hackathon/ink-ibm"}}}'
```

## Layout

```
src/
  main.rs        Entry point + transport setup (stdio/http CLI) + auth middleware
  lib.rs         Library facade (integration tests)
  handler.rs     InkServer type + ServerHandler impl (get_info, resources) + tool/prompt macros
  engine.rs      Engine bridge (analysis/graph/optimize) + progress notifications
  reporting.rs   File-based reporting (JSON + markdown summary)
  tools/         One file per tool (analyze, graph, optimize) + ToolRouter
  prompts/       One file per prompt (orchestrate_agent) + PromptRouter
  resources/     Resource helpers (ink://analysis/{root}, ink://graph/{root})
tests/
  integration_tests.rs   E2E: spawns the binary, drives raw JSON-RPC over stdio
run-mcp-inspector.bat   Launches MCP Inspector with reporting enabled
```