# General Rules — All Modes

These rules apply to every Bob mode in this repository.

## Commit convention
All commits must use **Conventional Commits** format (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`).

## Never modify
- `mcp/src/lib.rs` for production logic — it exists only to re-export `InkServer` for integration tests.
- Engine crate JSON output field names (snake_case serde) — the extension's `mcpMapping.ts` depends on this shape; changing names is a breaking contract change.
- `docs/skills/ink-orchestration/SKILL.md` without understanding it is embedded verbatim in the MCP `initialize` response — every connected agent sees it immediately.

## CI gate requirements
Before any Rust change is complete, these must pass locally:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Before any TypeScript change is complete:
```bash
cd extension && npm run compile
```
The CI workflow (`ci.yml`) runs all four as required checks; failing any means the PR will not pass.

## Adding a new VS Code webview panel
Four things are always required together:
1. A `renderXxxPage()` function in `extension/src/webviews/` using `renderWebviewPage()` or raw HTML with a nonce CSP
2. A provider or panel class in `extension/src/providers/`
3. Registration in `extension.ts` `activate()` + `context.subscriptions.push(...)`
4. A corresponding command or view entry in `extension/package.json` `contributes`
