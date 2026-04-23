---
id: TASK-0127
title: >-
  READ-10: CommandRunner::run_raw silently ignores composite `parallel = true`
  in single-command path
status: To Do
assignee: []
created_date: '2026-04-21 21:28'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:411-421`

**What**: `CommandRunner::run_raw` inspects `c.fail_fast` on a composite but never checks `c.parallel`. When the user runs `ops --raw <composite>` and the composite has `parallel = true` in `.ops.toml`, it silently runs sequentially with no warning.

The sibling fix in TASK-0125 added a `tracing::warn!` in the *multi-command* raw path (`crates/cli/src/run_cmd.rs:81-85`), but the single-command path goes through `run_command` → `run_command_raw` → `CommandRunner::run_raw`, which bypasses that warning.

**Why it matters**: `--raw` silently changes execution semantics for configs that depend on parallel execution (e.g. `verify = { commands = [...], parallel = true }`). A user profiling or scripting around parallel runs gets serialized timing with no diagnostic, exactly the regression TASK-0125 was filed to prevent — just on the other code path.

<!-- scan confidence: candidates to inspect -->
- `crates/runner/src/command/mod.rs:411-421` (`run_raw`)
- `crates/cli/src/run_cmd.rs:170-177` (`run_command_raw`)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When `CommandRunner::run_raw` (or its caller) encounters a composite with `parallel = true`, a `tracing::warn!` is emitted matching the phrasing in `run_cmd.rs:82-84`
- [ ] #2 Add a regression test covering `ops --raw <composite-with-parallel>` that asserts the warning is emitted
- [ ] #3 No existing behavior regresses: sequential composites and leaf commands under `--raw` still produce no spurious warning
<!-- AC:END -->
