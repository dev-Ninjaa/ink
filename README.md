# Ink

**Intelligent orchestration runtime for IBM Bob Agent Mode.**

Ink gives coding agents a structured understanding of your repository over the
[Model Context Protocol](https://modelcontextprotocol.io): repository
intelligence, dependency graphs, token-budgeted context bundles, agent
scheduling, cache visibility, and execution reporting — served by a single
Rust binary (`ink_mcp`) that any MCP client can call.

Instead of re-reading raw workspace state on every request, an agent asks Ink:
*what is this repo, what depends on what, which files actually matter for this
task, and what work is already underway?*

---

## Installation

### npx (quickest)

```bash
npx mcp-ink
```

Downloads the platform binary on first run. No Rust toolchain needed.

### Standalone binary

Download from
[GitHub Releases](https://github.com/dev-Ninjaa/ink/releases) — platform
archives for Linux, macOS (Apple Silicon), and Windows.

### VS Code Extension

Install from the
[Marketplace](https://marketplace.visualstudio.com/items?itemName=ninja75.ink-server)
or download a `.vsix` from GitHub Releases. Platform-specific builds bundle the
MCP server automatically — install the extension and the server is ready.

### Docker / hosted

```bash
docker run --init -p 3000:3000 ghcr.io/dev-Ninjaa/ink:latest
# → http://localhost:3000/mcp
```

## Architecture

```
┌──────────────────────┐        MCP stdio          ┌─────────────────────────┐
│  Agentic IDEs        │ ◀────────────────────────▶ │                         │
│  IBM Bob · Claude    │   tools/call · resources   │   ink_mcp (Rust)        │
│  Code · Cursor       │        prompts             │   9 tools               │
└──────────────────────┘                            │                         │
                                                    │   process-local state:  │
┌──────────────────────┐        MCP stdio or        │   agents · cache · runs │
│  VS Code extension   │ ◀────────────────────────▶ │                         │
│  (human cockpit)     │   HTTP (shared server)     └───────────┬─────────────┘
└──────────────────────┘                                        │ path deps
                                                    ┌───────────▼─────────────┐
                                                    │      Engine crates      │
                                                    ├─────────────────────────┤
                                                    │ repository_intelligence │
                                                    │ dependency_graph        │
                                                    │ context_optimizer       │
                                                    └─────────────────────────┘
```

Two kinds of consumers, one protocol:

- **Agents** (IBM Bob, Claude Code, Cursor) call the MCP tools to plan and
  execute work against real repository structure.
- **Humans** use the VS Code extension as an observability cockpit — point it
  at the same shared HTTP server to watch live agent activity while Bob works.

## Repository layout

| Path | What it is |
|------|------------|
| [`crates/repository_intelligence`](crates/repository_intelligence) | Parallel repo scanner: languages, frameworks, entry points, modules, import relationships |
| [`crates/dependency_graph`](crates/dependency_graph) | File/module dependency graph: cycles, central files, reachability |
| [`crates/context_optimizer`](crates/context_optimizer) | Token-budget context selection: chosen files, dropped files, token metrics |
| [`mcp`](mcp/README.md) | `ink_mcp` rmcp server — stdio (default) and Streamable HTTP transports |
| [`extension`](extension/README.md) | VS Code extension: dashboard, analytics, agents, cache views, interactive dependency graph panel |
| [`npm/ink-mcp`](npm/ink-mcp) | npm wrapper (`mcp-ink`) — detects platform, downloads the right binary, forwards CLI args |

## Quick start

Prerequisites: Rust ≥ 1.75 (stable), Node.js or Bun, VS Code (for the extension).

```bash
# Or via npx (no Rust needed)
npx mcp-ink

# Build + test everything
cargo test                          # engine crates + MCP server (150+ tests)

# Run the MCP server locally over stdio
cargo run -p ink_mcp -- --transport stdio

# Or serve it over Streamable HTTP
cargo run -p ink_mcp --features http -- --transport http --addr 0.0.0.0:3000
```

### Extension development

```bash
cd extension
bun install            # or npm install
npm run compile
```

Press **F5** in VS Code to launch the Extension Development Host. The extension
auto-detects a bundled `bin/ink_mcp`, then a sibling
`../mcp/target/{debug,release}/ink_mcp` build, then `ink.mcpServer.command` on
PATH.

## Configure

Add to your agent's MCP config (IBM Bob, Claude Code, Cursor, etc.):

### Local (npx — recommended)

```json
{
  "mcpServers": {
    "ink": {
      "command": "npx",
      "args": ["-y", "mcp-ink"]
    }
  }
}
```

### Local (direct binary)

```json
{
  "mcpServers": {
    "ink": {
      "command": "/path/to/ink_mcp",
      "args": ["--transport", "stdio"]
    }
  }
}
```

### Remote (Streamable HTTP)

```json
{
  "mcpServers": {
    "ink": {
      "url": "https://your-host.example.com/mcp"
    }
  }
}
```

No authentication — anyone with the URL can use the server.

## Tools

| Tool | Description |
|------|-------------|
| `analyze_repository` | Repository Intelligence JSON: files, languages, frameworks, entry points, modules, import relationships |
| `build_dependency_graph` | Dependency graph JSON: nodes, edges, cycles, central files, reachability |
| `optimize_context` | Token-budgeted context bundle: selected files, dropped files, token metrics |
| `schedule_agents` | Schedule orchestration agents derived from entry points (capped by `max_agents`) |
| `advance_agents` | Advance agent progress (default 25%), auto-complete at 100% |
| `list_agents` | List scheduled agents grouped by status |
| `get_cache_stats` / `clear_cache` | Inspect or clear per-repository analysis cache records |
| `generate_report` | Execution report: timeline events and runtime statistics |

Resources: `ink://analysis/{root}`, `ink://graph/{root}` ·
Prompt: `orchestrate_agent` (analyze → graph → optimize pipeline).

## Teaching agents to use Ink

Connecting is enough for tool *access*; agents orchestrate better with
*workflow* knowledge:

- **Automatic:** the server embeds a compact workflow card in its MCP
  `instructions` field — every connected agent sees it on `initialize`.
- **Full skill:** copy
  [`docs/skills/ink-orchestration/SKILL.md`](docs/skills/ink-orchestration/SKILL.md)
  into your workspace as `SKILL.md` (or reference it from your agent's
  config). It contains the decision table (which tool for which situation),
  pipeline recipes, parameter guidance, output-reading tips, and
  anti-patterns.

## CI/CD

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| **CI** | Every push / PR | `cargo fmt`, `clippy`, `cargo test`, extension `tsc` + tests |
| **Docker** | Push to `main` or `v*` tag | Build image, smoke-test MCP handshake, push to `ghcr.io` |
| **Release** | `v*` tag push or manual dispatch | Build binaries (Linux, macOS ARM, Windows), VSIX packages, create GitHub Release with SHA256 checksums |
| **npm publish** | After Release completes, or manual | Publish `mcp-ink` wrapper to npm |

To ship a release:

```bash
# 1. Bump version in extension/package.json and npm/ink-mcp/package.json
# 2. Commit, push, tag
git tag v0.4.0
git push origin v0.4.0
# 3. GitHub Actions builds binaries, VSIX packages, and creates the release.
# 4. npm publish workflow publishes mcp-ink to npm automatically.
```

Or trigger manually from the **Actions** tab using **workflow_dispatch**.

## Development

```bash
cargo test                        # full Rust suite (unit + integration + E2E)
cargo clippy --all-targets        # lint
cargo test -p ink_mcp             # server-only, incl. raw JSON-RPC E2E tests
cd extension && npm test          # extension unit tests (headless VS Code)
```

Conventions: Conventional Commits; Rust changes need green `cargo test` +
`clippy`; TS changes need green `tsc`. Server-side JSON uses `snake_case`
(engine serde), mapped to `camelCase` models in the extension.

## Status

**v0.3.1 — shipped:**

- 9 MCP tools: analyze, graph, optimize, schedule, advance, list, cache, report
- Persistent orchestration state (`INK_STATE_DIR` — agents, cache, runs survive
  restarts)
- stdio + Streamable HTTP transports
- VS Code extension: dashboard, analytics, agents, interactive graph panel,
  optimize panel, HTTP transport mode
- Platform binaries on GitHub Releases (Linux, macOS ARM, Windows)
- npm package `mcp-ink` — `npx mcp-ink` gets you running in seconds
- Docker image on `ghcr.io` with healthcheck
- Full CI/CD: lint → test → build → release → npm publish
- Agent skill file with decision table and pipeline recipes

**Planned:**

- Duplicate-work detection when scheduling overlapping agent tasks
- Result aggregation and structural verification of parallel edits
- npm wrapper with auto-update support

## License

MIT OR Apache-2.0 — see crate manifests.
