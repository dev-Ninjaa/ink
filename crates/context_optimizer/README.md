# context_optimizer

The Smart Context Optimizer for Ink: turn a full repository analysis into a
compact, deterministic context bundle for a specific developer request, so the
LLM only receives the files it actually needs.

Sits on top of [`repository_intelligence`](../repository_intelligence) (the
analysis) and [`dependency_graph`](../dependency_graph) (optional structural
signals), reads file content straight from disk, and produces serializable
output with selection reasons, dedup groups, token accounting and reduction
metrics.

## Quick start

```rust
use context_optimizer::{optimize_context, ContextRequest};
use dependency_graph::analyze_dependencies;
use repository_intelligence::analyze_repository;

let analysis = analyze_repository("/path/to/repo")?;
let graph = analyze_dependencies(&analysis);

let request = ContextRequest {
    query: "auth".to_string(),
    max_tokens: Some(8_000),
    ..Default::default()
};

let context = optimize_context(&analysis, Some(&graph), &request)?;

// Machine-readable JSON.
let bytes = context_optimizer::output::json::to_json(&context)?;

// Human-readable markdown report.
let report = context_optimizer::render_report(&context);
```

## Features

- **Relevant file selection** — files are ranked by query relevance using
  path tokens, filename stems (camelCase-aware), module membership, language
  names, entry points, graph centrality and reachability.
- **Content eligibility gate** — binary/media assets and generated lockfiles
  (`package-lock.json`, `Cargo.lock`, `yarn.lock`, ...) are excluded up front
  so the token budget is spent on real source.
- **Relevance threshold** — an optional `min_relevance` drops files whose
  normalized relevance falls below the requested floor.
- **Token estimation** — deterministic ~4-chars-per-token estimates per file
  and in total.
- **Budget pruning** — optional `max_tokens` / `max_files` caps drop the least
  relevant overflow with a per-file audit trail.
- **Near-duplicate removal** — MinHash LSH shingling collapses repeated or
  generated copies into a representative with reported similarity.
- **Full manifest** — every selection reason, drop reason, dedup group, token
  count and reduction percentage, as JSON and Markdown.
- **Deterministic** — identical repository + request produce byte-identical
  output across runs.

## Requirements

Rust 1.95+ (see workspace `Cargo.toml`).

## Test

```sh
cargo test --workspace
```

## Benchmark

```sh
cargo bench --bench context_optimizer
```

## Documentation

Design decisions, architecture and validation results live in
[`docs/context_optimizer_final.md`](docs/context_optimizer_final.md).

## Example

```sh
cargo run -p context_optimizer --example optimize -- /path/to/repo "auth login"
```