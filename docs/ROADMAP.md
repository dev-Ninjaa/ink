# Ink — Roadmap & Workflow Plan

Living list of what comes next, ranked by hackathon impact. Items marked
**done** are shipped; everything else is scoped but unbuilt.

---

## Overview

### 🎯 Product gaps (make existing claims true)

| # | Item | Effort | Status |
|---|------|--------|--------|
| 1 | Surface `optimize_context` results in the extension webview | S | **done** |
| 2 | Persistent server state (`INK_STATE_DIR`) | M | **done** |
| 3 | Extension transport setting → attach to shared HTTP server | S | **done** |
| 4 | Agent progression (`advance_agents` tool, progress %) | S | **done** |
| 5 | Deduplicate identical tasks in `schedule_agents` | XS | planned |
| 14 | Hosted-path sandbox (`INK_ALLOWED_ROOTS`) so shared-server callers cannot probe arbitrary filesystem paths | S | planned |

### 🔧 CI/CD tier

| # | Item | Effort | Status |
|---|------|--------|--------|
| 6 | `docker.yml`: build → ghcr.io → container smoke test | S | **done** |
| 7 | `release.yml`: multi-platform binaries + VSIX on tag | M | planned — **details below** |
| 8 | Scheduled security: `cargo audit` + Dependabot | XS | **done** — **details below** |
| 9 | MSRV job + `cargo-deny` license/advisory gate | XS | planned — **details below** |

### ✨ Polish

| # | Item | Effort | Status |
|---|------|--------|--------|
| 10 | Graph panel v2: cycle highlighting, centrality sizing, filter | M | planned |
| 11 | Status-bar MCP health indicator | XS | planned |
| 12 | Landing-page honesty pass + demo GIFs | S | planned |
| 13 | VS Code walkthrough for first-run setup | S | planned |

### ✅ Done

Monorepo consolidation · full green test suite · interactive dependency graph
panel · honest command results · Dockerfile (hardened, non-root) · monorepo
README · `ci.yml` quality gate passing on GitHub Actions · `docker.yml`
(build → smoke test → ghcr.io publishing) · scheduled security (`security.yml`
nightly cargo audit + Dependabot across cargo/npm/actions, with tuned ignore
rules for incompatible majors: typescript ≥ 6, rmcp ≥ 3, criterion ≥ 0.6) ·
`compose.yaml` with healthcheck and MCP Inspector profile · `/health`
unauthenticated probe endpoint · Render deployment pipeline (`deploy.yml`,
auto-fires after verified Docker runs, manual dispatch, live and verified) ·
agent skill layer (compact workflow card in MCP `initialize` instructions +
installable `docs/skills/ink-orchestration/SKILL.md`).

**Recommended sprint order:** 1 → 4 → 3 (one F5 demo session) → 7 → 10.
Item #14 pairs naturally with any hosted-demo work.

---

## 7 — `release.yml`: tag-driven release pipeline

**Trigger:** pushing a SemVer tag (`git tag v0.2.0 && git push origin v0.2.0`).

One workflow produces every distributable artifact and attaches it to a
GitHub Release:

```
                    ┌── job: binaries (matrix) ──┐
git tag v0.2.0 ──▶  │ windows-latest → ink_mcp-x86_64-pc-windows-msvc.zip
                    │ macos-14       → ink_mcp-aarch64-apple-darwin.tar.gz
                    │ macos-13       → ink_mcp-x86_64-apple-darwin.tar.gz
                    │ ubuntu-22.04   → ink_mcp-x86_64-unknown-linux-gnu.tar.gz
                    ├── job: extension → ink-0.2.0.vsix
                    ├── job: docker    → ghcr.io/dev-ninjaa/ink-mcp:v0.2.0
                    └── job: publish   → GitHub Release (all artifacts)
                                        ↳ optional: Marketplace + Open VSX
```

Sketch of the core jobs:

```yaml
on:
  push:
    tags: ["v*"]

jobs:
  binaries:
    strategy:
      matrix:
        include:
          - { os: windows-latest, target: x86_64-pc-windows-msvc }
          - { os: macos-14,       target: aarch64-apple-darwin }
          - { os: macos-13,       target: x86_64-apple-darwin }
          - { os: ubuntu-22.04,   target: x86_64-unknown-linux-gnu }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: "${{ matrix.target }}" }
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release -p ink_mcp --target ${{ matrix.target }}
      - run: ./scripts/package-release.sh ${{ matrix.target }}   # tar/zip the binary
      - uses: actions/upload-artifact@v4
        with: { name: ink_mcp-${{ matrix.target }}, path: dist/ }

  release:
    needs: [binaries, vsix]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - run: gh release create "$GITHUB_REF_NAME" --generate-notes dist/*
        env: { GH_TOKEN: "${{ github.token }}" }
```

Optional publish jobs need one-time secrets: `VSCE_PAT` (Azure DevOps token
for the Marketplace) and `OVSX_PAT` (Open VSX).

**Why it matters:** users without a Rust toolchain get a ready binary; a
follow-up extension feature can auto-download the right artifact on first run;
judges can install in seconds instead of compiling.

## 8 — Scheduled security: `cargo audit` + Dependabot ✅ shipped

> Shipped as `.github/workflows/security.yml` + `.github/dependabot.yml`.
> Lesson from the first Dependabot wave: 13 PRs in one hour, of which
> TypeScript ≥ 6, rmcp ≥ 3, and criterion ≥ 0.6 genuinely broke the build —
> now covered by permanent `ignore:` rules so they resurface only when we
> choose to migrate.

Two complementary pieces of zero-maintenance hygiene:

**`cargo audit`** scans `Cargo.lock` against the [RustSec Advisory
Database](https://rustsec.org/) — known CVEs, unmaintained crates, yanked
releases. Run it nightly so supply-chain disclosures surface without anyone
remembering to check:

```yaml
on:
  schedule: [{ cron: "0 6 * * *" }]     # daily, off-peak
  workflow_dispatch:

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with: { token: "${{ github.token }}" }
```

**Dependabot** (`.github/dependabot.yml`) opens ready-to-merge PRs that bump
outdated dependencies across all three ecosystems we use:

```yaml
version: 2
updates:
  - { package-ecosystem: cargo,         directory: "/",            schedule: { interval: weekly } }
  - { package-ecosystem: npm,           directory: "/extension",   schedule: { interval: weekly } }
  - package-ecosystem: github-actions
    directory: "/"
    schedule: { interval: weekly }
```

Division of labor: **audit finds vulnerable *versions you already have***;
**Dependabot keeps you current so fewer vulnerabilities accumulate**. Both
cost minutes to set up and signal engineering maturity during judging.

## 9 — MSRV job + `cargo-deny` policy gate

Unlike #8 (scheduled monitoring), these are **per-PR gates** — they fail the
check run before merge.

**MSRV job.** The workspace declares `rust-version = "1.75"`, but nothing
verifies it. If a dependency bump quietly requires Rust 1.85, every user on a
distro toolchain gets a cryptic error — discovered by them, not us. The fix is
one extra CI job:

```yaml
  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.75        # exact minimum we promise
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets
```

If this goes red after a dependency update, we either pin the dep back or
honestly raise the declared MSRV — a conscious decision instead of an
accident. (First run may reveal that `rmcp` already exceeds 1.75; then we
update `rust-version` to reality.)

**`cargo-deny`.** One tool enforcing four policies over the final dependency
graph, configured in a committed `deny.toml`:

| Check | What it blocks |
|-------|----------------|
| `licenses` | Incompatible licenses entering an MIT OR Apache-2.0 project |
| `advisories` | RustSec-listed vulnerable versions (per-PR companion to #8's nightly sweep) |
| `bans` | Duplicate major versions of the same crate bloating the binary |
| `sources` | Dependencies from unknown registries/git URLs |

```yaml
      - uses: EmbarkStudios/cargo-deny-action@v2
```

Single line in CI; the action ships a sane default config.

**Why both:** MSRV protects *users' compilers*, cargo-deny protects *users'
supply chain*. Together they turn two silent failure modes into loud,
actionable PR failures.
