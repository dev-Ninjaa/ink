---
name: ink-orchestration
description: >-
  Use the Ink MCP server (ink_mcp) for repository-aware agent work: analyzing
  codebases, planning multi-file edits against real dependency structure,
  filling context windows with only relevant files, scheduling parallel
  agents, and reporting execution. Trigger whenever a task involves
  understanding or changing an unfamiliar repository.
version: 1.0.0
---

# Ink Orchestration

Ink turns a repository into structured intelligence you can query over MCP:
what the repo contains, what depends on what, which files matter for a task,
and what parallel work looks like. One server, eight tools, three artifacts.

## Tool decision table

| Situation | Call first | Why |
|-----------|-----------|-----|
| Unfamiliar repository, any task | `analyze_repository` | Languages, frameworks, entry points, modules — your map |
| Planning cross-file edits | `build_dependency_graph` | Cycles + central files = blast radius before you touch anything |
| Big repo, small context window | `optimize_context` | Token-budgeted file selection; read only what's relevant |
| Work is parallelizable by component | `schedule_agents` | Derives one agent per entry point, capped and coordinated |
| Re-analyzing a root you already scanned | `get_cache_stats` | Hit rate tells you if analysis is already warm |
| Task finished / checkpoint | `generate_report` | Timeline + runtime statistics of everything that ran |

## Pipeline recipes

**Refactor / feature (default):**
```
analyze_repository → build_dependency_graph → optimize_context(task query)
→ make edits → generate_report
```

**Codebase onboarding:** `analyze_repository` → `build_dependency_graph`.
Skim entry points (confidence-sorted), then central files.

**Pre-edit risk check:** `build_dependency_graph` alone. If target files sit
on cycles or are listed central, widen review to their dependents.

**Parallel batch:** `analyze_repository` → `schedule_agents(max_agents=N)`.
Work each active agent's task; `list_agents` shows current states.

## Parameter guidance

- `optimize_context`: set `max_tokens` to roughly half the model window minus
  prompt overhead. Smaller queries → sharper selection.
- `schedule_agents`: keep `max_agents` ≤ number of entry points returned by
  analysis — extra capacity is wasted. Set `parallelism_enabled=false` when
  edits may conflict (agents then queue as pending).
- All tools take `root`: absolute repository path.

## Reading outputs

- `entry_points[]`: sorted by confidence (0.98 = canonical `src/main.rs`).
  The `heuristic` label says why it was chosen.
- `relationships[]`: import edges; `resolved: true` means the target file was
  found in-repo — trust these edges, ignore unresolved ones.
- Graph `cycles[]`: edit files in the same cycle together or compilation
  breaks.
- Central files: high fan-in/out — highest-risk edit targets.
- `optimize_context` result: `selected_files` earn your attention;
  `dropped_files` were deliberately excluded as noise — don't fetch them back
  without cause.

## Anti-patterns

- **Re-analyzing unchanged roots.** Check `get_cache_stats` first; analysis
  state persists per server process.
- **Editing central files without checking dependents.** Pull the graph
  first; a "small rename" can ripple across modules.
- **Blind `schedule_agents`.** It derives tasks from entry points — analyze
  first so you know what those entry points are.
- **Skipping optimize_context on large repos.** Reading raw files wastes
  context that the optimizer would have spent on relevant symbols.

## Errors

| Message | Meaning | Fix |
|---------|---------|-----|
| `repository root '...' does not exist or is not a directory` | Bad path | Send an absolute path that exists |
| `Ink requires an open workspace...` (extension side) | No folder open | Open a project first |

## Quick reference

Tools: `analyze_repository` · `build_dependency_graph` · `optimize_context`
· `schedule_agents` · `list_agents` · `get_cache_stats` · `clear_cache` ·
`generate_report`
Resources: `ink://analysis/{root}`, `ink://graph/{root}`
Prompt: `orchestrate_agent` — one-shot pipeline instruction for
analyze → graph → optimize.
