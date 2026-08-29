# repository_intelligence

Production-grade repository intelligence engine for Ink: scan any repository and
produce a deterministic, serializable analysis of its languages, frameworks,
build/metadata, entry points, import relationships and logical modules.

Fast (parallel scan, regex-only extraction — no tree-sitter), dependency-light,
and designed to power the Ink context engine, MCP servers and IDE grammars on
arbitrary codebases.

## Quick start

```rust
use repository_intelligence::Analyzer;

let analysis = Analyzer::with_defaults()
    .analyze("/path/to/repo")?;

// Machine-readable JSON.
let bytes = repository_intelligence::json::to_json(&analysis)?;

// Human-readable markdown report.
let report = repository_intelligence::render_report(&analysis);
```

## Features

- **Parallel scanning** with `ignore` (gitignore-aware) or sequential `walkdir`
  backends; deterministic, path-sorted output either way.
- **Language detection** for Rust, JavaScript/TypeScript, Python, Go, Java,
  C#, C, C++, JSON, YAML, TOML and Markdown (extension + well-known filenames).
- **Framework detection** driven by manifests: Next.js, React, Express, NestJS,
  Vite, FastAPI, Flask, Django, Axum, Actix, Rocket.
- **Metadata detection**: package managers (pnpm, npm, yarn, cargo, poetry,
  uv, pipenv, go, …), build systems, CI systems, Docker detection.
- **Entry-point detection** for rooted layouts, package `main`/`bin` fields,
  Rust `src/bin`, Go `cmd/*/main.go` and Python `main.py`/`manage.py`.
- **Import extraction** (regex-based, tree-sitter-free) for Rust, JS/TS and
  Python relationships, including alias resolution for `@/`, `@`, `~/`, `~`
  against project `src`.
- **Module discovery**: feature folders, layered architecture, and monorepo
  nests (`apps`, `packages`, `crates`, `libs`, `modules`).
- **Criterion benchmarks** targeting < 2 s for 10,000-file repositories.

## Requirements

Rust 1.95+ (see workspace `Cargo.toml`).

## Test

```sh
cargo test --workspace
```

## Benchmark

```sh
cargo bench --bench repository_analysis
cargo bench --bench repository_analysis --features heap-profiling  # dhat
```

## Documentation

Design decisions, architecture and benchmark results live in
[`docs/repository_intelligence_report.md`](docs/repository_intelligence_report.md).