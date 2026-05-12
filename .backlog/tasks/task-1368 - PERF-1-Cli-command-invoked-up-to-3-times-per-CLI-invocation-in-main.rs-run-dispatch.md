---
id: TASK-1368
title: >-
  PERF-1: Cli::command() invoked up to 3 times per CLI invocation in main.rs
  run()/dispatch()
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 21:34'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:188, 194, 268`

**What**: `run()` calls `Cli::command()` at line 188 (top-level help branch) AND at line 194 (parse branch). Then `dispatch()` calls it a third time at line 268 in the `None` (no-subcommand) help-print branch. Each `Cli::command()` invocation walks the full clap derive metadata and rebuilds the command tree from scratch — there is no internal cache. For a single CLI invocation taking the parse path, the tree is built twice (line 194 + the potential dispatch line 268). For the top-level-help path, it is built twice (line 188 + line 268 if hit — but that's a different branch). At minimum, lines 188 and 194 share the same `Cli::command()` shape and could share one builder; the help-fallback at 268 could clone or re-use rather than rebuild.

**Why it matters**: Startup latency on every `ops` invocation. The clap command tree is non-trivial (Cli + 10+ subcommand enums, each with derive args/about/help text). This adds measurable startup cost to a hot CLI binary that runs on every shell prompt for users with hook integration.

**Candidates to inspect**:
- `crates/cli/src/main.rs:188` — `hide_irrelevant_commands(Cli::command(), detected_stack)` for top-level-help branch
- `crates/cli/src/main.rs:194` — `hide_irrelevant_commands(Cli::command(), detected_stack)` for parse branch
- `crates/cli/src/main.rs:268` — `hide_irrelevant_commands(Cli::command(), detected_stack)` in dispatch `None` arm

Note: this is distinct from TASK-1318 (which is about `validate_command_name` rebuilding the tree per inquire keystroke); this one is the once-per-invocation overhead on the hot startup path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cli::command() is invoked at most once per CLI invocation (or the cost is documented and benchmarked)
- [ ] #2 the three call sites in run()/dispatch() share a single built tree, or each call site uses a cached/cloned Command
<!-- AC:END -->
