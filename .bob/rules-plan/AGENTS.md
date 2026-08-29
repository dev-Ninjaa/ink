# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Non-Obvious Architectural Constraints

- **Engine crates are a strict one-way dependency chain**: `context_optimizer` → `dependency_graph` → `repository_intelligence`. No cycles. `mcp` depends on all three as path deps. The extension depends on none of them (communicates only over JSON via MCP).
- **`InkServer` is `Clone`** — the HTTP transport creates one instance per session via the factory closure `|| Ok(InkServer::new())`. State is shared through `Arc<Mutex<>>`, not by cloning it.
- **Process-local state means HTTP and stdio sessions do not share state** unless they happen to be the same process. Multiple concurrent clients on the HTTP transport share state; stdio spawns are isolated.
- **`schedule_agents` clears all existing agents** before scheduling new ones (calls `self.agents.clear()`). There is no additive scheduling — every call is a full replacement.
- **The `advance_agents` promotion logic is implicit**: when no agent is active after an advance pass, the first pending agent is automatically promoted. This is stateful sequencing behavior, not explicit queuing.
- **Extension binary resolution is priority-ordered**: explicit setting → bundled `bin/ink_mcp` (VSIX) → sibling `mcp/target/{debug,release}/ink_mcp` (dev) → PATH. Marketplace VSIX installs get a bundled platform-specific binary; dev installs fall through to the sibling build.
- **`now_iso8601()` in `state.rs` produces a non-standard timestamp** (`1970-01-01T00:00:<secs%60>Z`) — downstream consumers that parse this as a real date will get wrong values. Do not add new code that depends on this timestamp being accurate.
- **Extension webview tests require a real VS Code instance** — `@vscode/test-electron` downloads/reuses VS Code 1.92.0. There is no unit-testable layer below the VS Code API boundary; all unit tests in `suite/` rely on the live extension host.
- **Benchmarks (`[[bench]]` in each crate Cargo.toml) use Criterion with `harness = false`** — run with `cargo bench -p <crate>`, not `cargo test`.
- **`mcp/src/reporting.rs` timestamp uses Howard Hinnant's `civil_from_days`** — no `chrono` dependency. Adding time-zone awareness requires pulling in a crate; the current impl is intentionally dependency-free.
