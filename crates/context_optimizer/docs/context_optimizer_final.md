# Context Optimizer Final Report

## Executive Summary

The Ink Smart Context Optimizer is implemented as a dedicated Rust crate at `crates/context_optimizer`. It consumes `repository_intelligence::RepositoryAnalysis` directly, optionally augments relevance signals with `dependency_graph::models::AnalysisResult`, reads file content from disk, and produces a compact, deterministic context bundle for a specific developer request.

The optimizer ranks files by query relevance, collapses near-duplicate content with a MinHash LSH pipeline, enforces an optional token/file budget, estimates tokens, and emits serializable JSON plus a human-readable Markdown report — everything the PRD's `OptimizeContext`, `EstimateTokens` and context-reduction targets require.

Final status: READY FOR THE MCP SERVER

## Architecture

- Crate: `context_optimizer`
- Upstream contract: `repository_intelligence::RepositoryAnalysis` (required), `dependency_graph::models::AnalysisResult` (optional)
- Output contract: serde-serializable `OptimizedContext`
- JSON adapters: `output::json::to_json`, `to_json_compact`, `to_value`
- Report adapter: `output::report::render_report`
- Profiling example: `cargo run -p context_optimizer --example optimize -- <repo-path> [query]`

Pipeline stages (orchestrated in `optimizer::ContextOptimizer::optimize`):

1. `rank_candidates` — include/exclude filters, then the content gate (binary/media + generated lockfile exclusion), per-file relevance scoring, and the optional `min_relevance` threshold.
2. `dedup::detect_near_duplicates` — MinHash LSH near-duplicate collapsing.
3. `pruner::Pruner` — token/file budget gate in relevance order.
4. Accounting — token summary, reduction metrics, warnings.

## Design

### Content eligibility gate (`gate`)

- Files are classified up front by path: `Source` (kept), `NonText` (binary/media — images, archives, executables) and `Generated` (lockfiles like `package-lock.json`, `Cargo.lock`, `yarn.lock`, `pnpm-lock.yaml`, minified bundles). Windows and POSIX paths classify identically.
- Gated files never reach scoring, so the token budget and reduction metrics are computed over real source only. Dropped entries carry `non_text` / `generated` reasons with per-file detail.
- The gate can be disabled via `OptimizerConfig::content_gate_enabled` (default `true`).

### Relevance scoring (`selector`)

- Path-token match `+3`, filename-stem match `+2`, module match `+1`, language match `+1.5`. Tokenization is camelCase-aware, so `DashboardProvider.ts` and `dashboard provider` produce the same tokens.
- Structural signals (gated by `structural_boost`): entry point `+2`, graph-central file `+1`, reachable-from-entry-points `+0.5`.
- Scores are normalized to `relevance ∈ [0,1]` and sorted deterministically (score desc, path asc).
- `include_paths` / `exclude_paths` support exact paths or directory prefixes.
- `min_relevance` (request-level, clamped to `0..=1`) drops candidates whose normalized relevance is below the floor with a `low_relevance` reason before dedup/pruning.

### Near-duplicate removal (`dedup`)

- Content is tokenized into hashed tokens (FNV-1a, fixed seeds), then 5-shingle windows form a bounded set per file.
- A 64-dim MinHash signature estimates Jaccard similarity; LSH banding buckets candidate pairs so only promising pairs are compared exactly.
- Pairs at/above `similarity_threshold` (default 0.8) are unioned; the first member in relevance order is the representative, the rest collapse with their similarity.
- Dedup scope is bounded by `max_dedup_candidates` (default 4096) or `max_files * 4` (min 128), so memory stays proportional to the top-ranked files.

### Budget pruning (`pruner`)

- Stateful gate offering candidates in relevance order; drops the overflow once `max_files` or `max_tokens` is reached, each with an audit detail.
- Content is read lazily and cached (`content_cache`), so only dedup-scope and selected files are ever read from disk.
- Token estimation uses `ceil(chars / 4)`, a deterministic model-agnostic approximation.

## Validation

- **Test suite:** 36 green (24 unit + 11 integration + 1 doc); integration tests run the full Repository Intelligence + Dependency Graph pipeline against tempdir repositories, including binary `.png` files and lockfiles.
- **Static checks:** `clippy --all-targets` 0 warnings; deterministic JSON pinned byte-for-byte in tests.
- **Real-repo validation** (`ink.extension`, query `dashboard`, `max_tokens: 8000`):

| Metric | Value |
|---|---|
| Files considered / selected | 44 / 22 |
| Dropped: non-text / generated / budget | 1 (`assets/icon.png`) / 1 (`package-lock.json`) / 22 |
| Tokens before / after | 22,832 / 7,987 |
| Token reduction | 65.0% |
| Within budget | true |
| Duration | ~0.28 s |

Query `dashboard` correctly surfaced `DashboardProvider.ts` and `DashboardWebview.ts` at relevance 1.00 (exact path + filename match), then entry point, central files and reachable files via graph signals. The gate removed `assets/icon.png` and `package-lock.json` before scoring — previously they consumed ~21k of the 43,798 pre-optimization tokens.

## Example

```sh
cargo run -p context_optimizer --example optimize -- /path/to/repo "auth login"
```

```rust
let request = ContextRequest {
    query: "auth".to_string(),
    max_tokens: Some(8_000),
    ..Default::default()
};
let context = optimize_context(&analysis, Some(&graph), &request)?;
let json = context_optimizer::output::json::to_json(&context)?;
```