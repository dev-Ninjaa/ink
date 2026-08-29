# Ink

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](./package.json)
[![VS Code](https://img.shields.io/badge/VS%20Code-%5E1.92.0-007ACC.svg)](https://code.visualstudio.com/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.5-3178C6.svg)](https://www.typescriptlang.org/)

AI orchestration visibility for VS Code workspaces.

Ink is a VS Code extension frontend for orchestration-oriented development workflows. Every operation is backed by the live `ink_mcp` server over the Model Context Protocol (stdio): the server performs repository analysis, dependency graph building, context optimization, agent scheduling, cache tracking, and report generation — no mock data.

## Features

- Dedicated Ink Activity Bar container with Dashboard, Analytics, Agents, and Cache views.
- Dashboard webview with workspace-aware orchestration metrics, including tokens saved, cache hit rate, parallel tasks, execution time, and context reduction.
- Analytics webview with execution timeline, cache statistics, and runtime statistics.
- Context Optimization panel: token-before/after, reduction percentage, selected files with relevance reasons, and dropped files.
- Interactive dependency graph panel: a force-directed SVG visualization with draggable nodes, pan, zoom, and hover highlighting of each file's connections. Entry points render in the accent color.
- Agents tree view grouped by active, completed, and pending agent status.
- Cache tree view with cache entry count, total cache size, hit rate, and individual cache entries.
- Workspace-aware behavior that prompts users to open a project before running workspace-dependent commands.
- Command palette entries for opening the dashboard, optimizing context, building a dependency graph, scheduling agents, generating a report, and clearing cache.
- Settings for cache, analytics, parallelism, maximum scheduled agents, and the MCP server command.
- Output channel logging through the `INK` channel.
- Live MCP integration: the extension spawns the `ink_mcp` binary via the official `@modelcontextprotocol/sdk` over stdio and maps its `analyze_repository`, `build_dependency_graph`, and `optimize_context` tools onto the runtime interface.
- Runtime, MCP, settings, state, health, workspace, and event abstractions prepared for future runtime integration.

## MCP Server Integration

The extension connects to the Rust `ink_mcp` server (see the sibling `mcp` directory) over stdio using the official `@modelcontextprotocol/sdk`:

- `analyze_repository` drives the Dashboard metrics.
- `build_dependency_graph` drives the dependency graph boundary.
- `optimize_context` drives context optimization.
- `schedule_agents` / `list_agents` drive agent scheduling and the Agents view.
- `get_cache_stats` / `clear_cache` drive cache visibility.
- `generate_report` drives the Analytics timeline and report.

All orchestration tools share process-local state on the server (agents, cache, run history), so the extension reflects live server activity rather than mock data.

The server command resolves in this order:

1. The `ink.mcpServer.command` setting if set.
2. An auto-detected sibling build at `../mcp/target/{debug,release}/ink_mcp` relative to the extension.
3. `ink_mcp` on `PATH`.

If the server cannot be launched, the extension continues to run and logs the failure to the `INK` output channel.

## Commands

| Command | Description |
| --- | --- |
| `Ink: Open Dashboard` | Opens the Ink Activity Bar container, focuses the Dashboard view, and refreshes dashboard metrics. |
| `Ink: Optimize Context` | Optimizes workspace context for the active workspace and opens a panel showing token savings, selected files, and what was pruned. |
| `Ink: Build Dependency Graph` | Builds the dependency graph for the active workspace and opens it in an interactive panel (drag, pan, zoom, hover to highlight connections). |
| `Ink: Schedule Agents` | Calls the runtime agent scheduling boundary and refreshes the Agents view. |
| `Ink: Generate Report` | Calls the runtime report generation boundary and refreshes the Analytics view. |
| `Ink: Clear Cache` | Calls the runtime cache clearing boundary and refreshes the Cache view. |

## Installation

Install from the VS Code Marketplace or Open VSX once the extension is published.

For local development:

```bash
npm install
npm run compile
```

Then open this repository in VS Code and press `F5` to launch an Extension Development Host.

## Quick Start

1. Open a workspace folder in VS Code.
2. Open the Ink Activity Bar container.
3. Review Dashboard metrics, Analytics timeline, Agents status, and Cache entries.
4. Run `Ink: Open Dashboard` from the Command Palette to return to the main Ink view.

## Usage

Ink is designed around workspace visibility. When no folder is open, views show a project prompt and workspace-dependent commands are hidden from the Command Palette.

When a folder is open, Ink shows runtime preview data across four views:

- Dashboard summarizes orchestration metrics.
- Analytics shows execution timeline and runtime statistics.
- Agents groups scheduled work by state.
- Cache summarizes cached entries and cache health.

- Runtime operation commands exercise typed runtime boundaries backed by the live MCP server.

## Architecture Overview

Ink uses a layered TypeScript architecture:

- `src/extension.ts` is the composition root for services, providers, commands, and VS Code context keys.
- `src/commands/` registers command handlers with logging, workspace checks, and error handling.
- `src/providers/` adapts VS Code webview and tree view APIs to runtime data.
- `src/webviews/` renders dashboard and analytics HTML with VS Code theme variables.
- `src/services/` contains runtime, settings, workspace, logging, health, state, and MCP abstractions.
- `src/services/McpClient.ts` is the SDK-backed MCP client that spawns `ink_mcp` over stdio.
- `src/services/McpRuntime.ts` maps every server tool onto the runtime interface.
- `src/mcp/mcpMapping.ts` converts engine JSON documents into extension data models.
- `src/models/` defines typed data contracts shared by the runtime and UI.
- `src/mocks/` provides the mock runtime and sample data used by tests and offline development.

Providers and commands depend on runtime interfaces rather than directly coupling to mock data, so a future runtime can be introduced behind the same contracts.

## Configuration

Ink contributes these settings:

| Setting | Default | Description |
| --- | --- | --- |
| `ink.enableCache` | `true` | Enables Ink cache features. |
| `ink.enableAnalytics` | `true` | Enables Ink analytics features. |
| `ink.enableParallelism` | `true` | Enables parallel agent orchestration features. |
| `ink.maxAgents` | `4` | Sets the maximum number of agents Ink may schedule. |
| `ink.mcpServer.command` | `""` | Command that launches the Ink MCP server. Empty auto-detects a sibling `ink.mcpServer` build or `ink_mcp` on `PATH`. |
| `ink.mcpServer.args` | `["--transport", "stdio"]` | Arguments passed to the Ink MCP server command (stdio mode). |
| `ink.mcpServer.transport` | `"stdio"` | `"stdio"` spawns a local binary; `"http"` attaches to a shared Streamable HTTP server. |
| `ink.mcpServer.url` | `""` | Streamable HTTP endpoint used when transport is `"http"` (e.g. your Render URL). |

## Development with the MCP server

Build the Rust server first so the extension can auto-detect it:

```bash
cd ../mcp
cargo build
```

Then launch the extension from this folder with `F5`. The `INK` output channel logs MCP connection details (tools discovered) and any server failures.

## Troubleshooting

If Ink views ask you to open a project, open a folder or workspace before using workspace-dependent commands.

If a command fails, open the `INK` output channel from **View: Toggle Output** and inspect the logged error. Check for a "Failed to connect to the Ink MCP server" message; if present, run `cargo build` in the sibling `mcp` directory (or set `ink.mcpServer.command` to a valid server binary).

If views do not update after changing workspaces or settings, reload the Extension Development Host and reopen the Ink Activity Bar container.

## Development

Install dependencies:

```bash
npm install
```

Start a TypeScript watch build:

```bash
npm run watch
```

Launch locally from VS Code with `F5`.

## Build

Compile the extension:

```bash
npm run compile
```

The compiled JavaScript is emitted to `out/`.

## Test

Run the extension test command:

```bash
npm test
```

The test script compiles the extension before launching the VS Code extension test runner.

## Packaging

Before packaging, confirm `package.json` metadata, marketplace icon, README, changelog, and repository links.

Package with `vsce` after installing it:

```bash
npm install -g @vscode/vsce
vsce package
```

For Open VSX, use `ovsx` after installing it:

```bash
npm install -g ovsx
ovsx package
```

## Release Process

1. Run `npm run compile`.
2. Run `npm test`.
3. Confirm `assets/icon.png` is referenced by `package.json`.
4. Review `CHANGELOG.md` and `docs/marketplace/release-description.md`.
5. Replace placeholder repository, bugs, and homepage metadata before publishing.
6. Package a `.vsix` and install it locally for smoke testing.
7. Publish to the VS Code Marketplace and Open VSX.

## License

No license file is included yet. Add a license before public distribution.
