---
id: TASK-2826
title: 'API-23: tracked_only bool in FixerOptions::new and discovery::discover is opaque at call sites'
status: Done
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 21:41'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/text-fixers/src/options.rs
  - extensions/text-fixers/src/discovery.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/doc_ids_cmd.rs
  - extensions/config-checkers/src/runner.rs
priority: low
ordinal: 1000
dedup_key: 'API-23:extensions/text-fixers/src/options.rs:FixerOptions::new'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/options.rs:35` (`FixerOptions::new`), `extensions/text-fixers/src/discovery.rs:161` (`discover`)

**What**: Both public entry points take a positional `tracked_only: bool` whose meaning the function name does not explain. Call sites read `FixerOptions::new(cwd, tracked)`, `FixerOptions::new(root.to_path_buf(), false)` (`src/tests.rs:31`), `discover(root, false)` (`crates/cli/src/doc_ids_cmd.rs:70`, `src/tests.rs:38`) — the `false` cannot be reviewed without opening the signature. The flag also selects between two genuinely different candidate sources (git index vs. filesystem walk) with a documented third behaviour (fallback), which is the shape API-23 says should be an enum.

**Why it matters**: The flag decides which of the user's files a destructive rewriter touches. A `Discovery::Walk` / `Discovery::Tracked`-style enum reads at the call site, cannot be confused with another bool, and leaves room for a future mode (e.g. staged-only) without a breaking `Option<bool>`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 FixerOptions::new and discovery::discover take a two-variant enum (e.g. Scope::Walk / Scope::Tracked) instead of tracked_only: bool
- [x] #2 All call sites in the workspace (cli, config-checkers, tests) pass the named variant

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.31 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
