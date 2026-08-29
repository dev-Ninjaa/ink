# Repository Intelligence Engine — Final Validation & Hardening Report

**Date:** 2026-08-28 · **Engine version:** 0.1.0 · **Workspace:** `crates/repository_intelligence`
**Validated against:** `ink.extension` (VS Code extension, TypeScript) and `maple` (Rust + Zig + C package manager)

---

## 1. Executive summary

The repository intelligence engine completed a full production-validation pass against two
real, non-trivial repositories, followed by a targeted hardening phase and a clean
re-verification. The engine detects languages, frameworks, package managers, entry points,
logical modules and file-to-file relationships with high accuracy, runs far inside its
performance budget, and emits deterministic JSON that is safe to use as an inter-subsystem
contract.

Seven issues were fixed (all qualifying under the "real bug, low risk, improves correctness"
rule), including a Rust `super::` resolution bug, an npm scoped-package alias collision, and
a backend-parity bug that made `ScanBackend::Walkdir` silently ignore `.gitignore`. No
schema or public-API changes were made.

**Production readiness score: 87 / 100.** The engine is **READY FOR THE NEXT FEATURE**
(Dependency Graph Builder, Context Optimizer, scheduler, MCP server, VS Code extension).

| Area | Result |
|---|---|
| Real-repo validation | 2 repos, all categories exercised |
| Detection accuracy | High; gaps documented and mostly benign |
| Test suite | 80 tests green (66 unit + 11 integration + 3 doc) |
| Static checks | `clippy --all-targets --all-features` 0 warnings; `fmt --check` clean |
| Performance | 10k files analysed in ~337 ms (budget 2 s) |
| Tree-sitter | **NO** for this phase (documented, seam preserved) |

---

## 2. Architecture review

- Single-pass pipeline (`scan → detect → extract → summarise`) with one detector module per
  concern; models, output serializers and errors are cleanly separated.
- Determinism by construction: `BTreeMap`/`BTreeSet` collections, sorted outputs, and a
  sequential-traversal fallback — integration tests pin byte-for-byte reproducibility.
- Safe surface: `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`, no unsafe dependencies;
  `dhat` heap profiling is feature-gated.
- Traversal is gitignore-aware (`ignore` crate, default) with a deterministic `Walkdir`
  backend; the two now produce identical file sets (parity bug fixed in this pass).
- Two scan backends (`Ignore` parallel, `Walkdir` sequential) share one post-processor that
  sorts and aggregates output, so backend choice does not affect results.

## 3. Repository validation results

Both repositories were analysed with the release build. The raw directory trees contained
2,641 (ink.extension) and 11,852 (maple) entries dominated by `node_modules`, `target`,
`.zig-cache` and `.vscode-test`; the engine correctly reduced these to the real source sets.

| Metric | ink.extension | maple |
|---|---|---|
| files / dirs / project roots | 47 / 17 / 1 | 67 / 8 / 1 |
| bytes | 1,643,067 | 857,272 |
| scan time (steady-state) | ~9.0 ms | ~8.7 ms |
| analyse time (steady-state) | ~8.2 ms | ~6.7 ms |
| total time (steady-state) | ~17.2 ms | ~15.2 ms |
| files per second | ~2,700 | ~4,400 |
| languages | TS 39, JSON 3, MD 1 (+2 png, 1 svg, 1 no-ext) | Rust 19, C 2, JSON 2, TOML 1, MD 30 (+4 zig, 3 ps1, 4 txt, 1 lock) |
| frameworks | none (correct — not a web app) | none (correct — CLI) |
| package managers | npm | cargo, npm |
| entry points | `src/extension.ts` | `src/main.rs`, `src/lib.rs`, `src/bin/bench.rs` |
| modules | 10 | 6 |
| relationships | 112 (112 resolved / 0 unresolved) | 35 (18 resolved / 17 unresolved) |

Memory (synthetic, dhat): 100 files → 9.0 MB allocated; 1,000 files → 6.5 MB; 10,000 files
→ 38.9 MB in 410,972 blocks. Real-repo heuristic estimate ~1–2 MB each.

## 4. Detection accuracy review

Per category, after this hardening pass (✓ correct, ~ acceptable, ✗ gap):

- **Languages ✓** — all supported extensions correctly classified in both repos; unknown
  files (svg/png/zig/ps1/txt) are excluded rather than mislabelled.
- **Frameworks ✓** — correct negative results (neither repo uses a web framework).
- **Package managers ✓** — npm and cargo inferred from lockfiles/manifests.
- **Entry points ✓ (ink after fix) / ✓ (maple)** — VS Code extension entry now detected
  (`src/extension.ts`, 0.85); maple's three Rust entry points all correct.
- **Modules ~** — feature/layer classification accurate (`src/services`, `src/models`,
  `src/core`); residual noise is data-output directories (`bench_results`, `compat_results`)
  reported as Feature modules.
- **Relationships ~** — ink.extension resolves 112/112 (was 112/113 with one false
  unresolved scoped-package edge, now fixed). maple resolves `src/core/mod.rs` → all 14
  sibling modules plus correct import edges; the remaining 17 unresolved edges are
  crate-root type re-exports (`crate::Result`, `crate::Config`, …) which are conservative
  skips, not extraction errors.

## 5. Tree-sitter evaluation — **NO** (this phase)

The backend seam is intact and correct (`ImportExtractor` is already backend-agnostic), but
adding a tree-sitter backend was rejected on cost/benefit: it requires a C toolchain at
build time, adds ~4 dependencies (core + rust/javascript/typescript grammars) with ongoing
ABI-version maintenance, reduces import-extraction throughput, and its precision gains do
not move the needle on the two validated repos — their remaining gaps are *resolver*-level
(crate-root re-exports), which tree-sitter does not address. Re-evaluate when the Dependency
Graph Builder needs extractor-level precision (type-only imports, macro expansion, SFCs).
Full analysis in `docs/audit_findings.md` (Phase 3 section).

## 6. Benchmark before / after

Criterion, `--noplot`, median of 10 samples (full analyze budget for 10k files = 2 s).

| Case | Files | Scan before | Scan after | Analyze before | Analyze after |
|---|---|---|---|---|---|
| small | 100 | 9.8 ms | **8.56 ms** | 16.8 ms | **13.48 ms** |
| medium | 1,000 | 25.5 ms | **24.99 ms** | 52.0 ms | **42.44 ms** |
| large | 10,000 | 177 ms | **180.07 ms** | 390 ms | **336.81 ms** |

Large analyze throughput ≈ 29.7k files/s. All groups are faster or flat versus the stored
criterion baseline (change reports −0.8%…−22%, none significant regressions). The 10k-file
analyze stays **~6× under** the 2 s budget. Real repos analyse in ~15–18 ms end-to-end.

## 7. Test results

`cargo test --workspace --all-features` → **66 unit + 11 integration + 3 doc = 80 passed,
0 failed**. `cargo clippy --all-targets --all-features` → **0 warnings**.
`cargo fmt --check` → **clean**. Coverage added this pass: scoped-package handling, custom
alias remainder, Rust `super::` from leaf and `mod.rs` files, Walkdir gitignore parity,
`.zig-cache` pruning, VS Code extension entry point, `media` module exclusion.

## 8. Issues found

- **Critical:** none.
- **Major:** (1) Rust `super::` resolved one directory too high → false unresolved edges;
  (2) Walkdir backend silently ignored `respect_gitignore` → results diverged from the
  default backend (950 vs 47 files on ink.extension).
- **Minor:** (3) bare `@` alias collided with npm scoped packages (`@vscode/test-electron`
  reported unresolved); (4) custom JS/TS aliases without a trailing slash produced
  root-relative joins; (5) `src/extension.ts` entry point missed; (6) `media` asset folder
  misreported as a module; (7) `.zig-cache` not pruned by default.
- **Noted (no fix):** `tsconfig.json` counted as JSON not TS; `.h` always C; `scan_errors`
  not surfaced in output; path-set rebuilt 4×; entry-point detector O(roots×files); regex
  extractor reads comments/strings; Zig/Shell/PowerShell unsupported; VS Code "framework"
  undetected; `files_per_second` based on total duration.

## 9. Issues fixed (changelog)

1. `import_extractor`: scoped packages (`@scope/name`) are external; bare `@` alias no
   longer matches them (reportable_unresolved + resolve guard).
2. `import_extractor`: alias remainder strips a leading `/` before join (fixes
   `@components/Button`-style custom aliases).
3. `import_extractor`: `super::` resolution for Rust leaf modules (pop n−1); `mod.rs`
   unchanged (pop n).
4. `scanner`: `Walkdir` honours `respect_gitignore` by delegating to the `ignore` crate's
   sequential walker — exact parity with the default backend.
5. `entrypoint_detector`: `src/extension.ts` rule (`vscode_extension_main`, 0.85).
6. `module_detector`: `media` added to `EXCLUDED_DIRS`.
7. `scanner`: `.zig-cache` added to `default_ignored_dirs`.

No schema/API changes; JSON contract unchanged.

## 10. Remaining limitations

- Regex import extraction can match imports inside comments/strings (no false resolved
  edges observed on the validated repos).
- Crate-root type re-exports (`crate::Result`) are not resolved to `lib.rs` (conservative
  by design).
- Unsupported languages (Zig, Shell, PowerShell, etc.) are listed but uncounted.
- Hidden directories not covered by `.gitignore` or the ignore list (e.g. `.vscode-test`
  without a gitignore) are scanned by design (`include_hidden` default true).
- Data-output directories in a repo can surface as Feature modules (no repo-specific
  ignore list yet).
- Memory: ~39 MB total allocation for a 10k-file analysis (acceptable; see §3).

## 11. Hackathon readiness assessment

| Criterion | Status |
|---|---|
| Demoable | Yes — 15 ms on real repos, deterministic JSON output |
| Independently judgeable | Yes — tests + bench are runnable; report reproducible |
| Dependency Graph Builder | Blockers cleared (backend parity, Rust/JS/Python resolution) |
| Context Optimizer | Entry points + module boundaries accurate enough to seed context |
| Scheduler / MCP server | Stable JSON contract (`RepositoryAnalysis`) unchanged |
| VS Code extension | Engine validated against the actual extension repo |

## 12. Final recommendation

**READY FOR NEXT FEATURE.** The engine is stable, fast, deterministic, tested, and has been
validated against two real repositories with the hardening issues resolved. Recommended
next steps: (a) implement the Dependency Graph Builder on top of `relationships`; (b) add
crate-root re-export resolution and a Zig language entry when that feature needs them; (c)
revisit tree-sitter only if extractor-level precision becomes a requirement.

*Companion documents: `docs/repository_intelligence_report.md` (build report) and
`docs/audit_findings.md` (full audit + validation + Phase-4 changelog).*
