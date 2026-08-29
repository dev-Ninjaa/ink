---
name: ink-engine-crate
description: >-
  Work on one of the three Ink engine crates: repository_intelligence,
  dependency_graph, or context_optimizer. Use when the task involves changing
  analysis logic, adding new models, or modifying output formats in crates/.
version: 1.0.0
---

# Working on Ink Engine Crates

## Crate responsibilities and boundaries

| Crate | Entry point | Takes | Returns |
|-------|------------|-------|---------|
| `repository_intelligence` | `analyze_repository(root)` | filesystem path | `RepositoryAnalysis` |
| `dependency_graph` | `analyze_dependencies(analysis)` | `&RepositoryAnalysis` | `AnalysisResult` |
| `context_optimizer` | `optimize_context(analysis, graph?, request)` | `&RepositoryAnalysis`, optional `&AnalysisResult`, `&ContextRequest` | `OptimizedContext` |

**Dependency direction is strict and one-way.** `dependency_graph` may import `repository_intelligence`. `context_optimizer` may import both. No reverse imports. `mcp` imports all three via path deps.

## Output stability rules
- All public model structs must derive `Serialize, Deserialize` and use `#[serde(rename_all = "snake_case")]` — the MCP server serializes them directly to JSON for clients.
- All collections in output structs must be **deterministically ordered** — `BTreeMap` for key-value maps, `.sort()` before returning `Vec`s. Integration tests assert byte-identical output across repeated runs.
- Never add `f64` fields that include timing/performance data to the stable comparison path — see `repeated_analysis_is_deterministic` test in `analyzer/mod.rs` for how this is handled.

## Testing patterns
- Unit tests live in `#[cfg(test)]` modules at the bottom of each source file.
- Use `tempfile::tempdir()` for all filesystem-based tests — never hardcode paths.
- The `write(root, rel, content)` helper pattern (seen in `analyzer/mod.rs` tests) is the idiomatic way to set up fixture files.
- Run a single crate's tests: `cargo test -p repository_intelligence <test_name>`
- Run benchmarks: `cargo bench -p <crate>` (uses Criterion, `harness = false`)

## Adding a new field to a model
1. Add the field to the struct in `crates/<crate>/src/models/`.
2. Add population logic in the analyzer/builder.
3. Add the field to the JSON output module in `crates/<crate>/src/output/json.rs` if it has a separate serialization path.
4. If the field appears in `AnalysisDocument` or `GraphDocument` on the extension side, add it to `extension/src/mcp/mcpMapping.ts`.
5. Run `cargo test --workspace` — determinism tests will catch ordering violations.

## `ContextRequest` parameters (for optimizer work)
- `query`: free-form text; empty query produces a warning and falls back to structural signals only.
- `include_paths` / `exclude_paths`: repo-relative path prefixes, stripped of leading `./` and trailing `/`.
- `max_tokens` / `max_files`: optional caps; `None` means unbounded.
- `min_relevance`: `0.0..=1.0`, clamped; files below threshold get `DroppedReason::LowRelevance`.

## `unsafe` code
All three crates declare `#![forbid(unsafe_code)]`. Do not remove this. Any code requiring unsafe must go into a separate crate.
