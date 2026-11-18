---
id: TASK-2702
title: 'API-14: CommandRunner::run and run_raw # Errors claim step build/spawn failures return Err'
status: To Do
assignee: []
created_date: '2026-11-18 18:31'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2842'
modified_files:
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/sequential.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/runner/src/command/mod.rs:CommandRunner::run'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:560` (`CommandRunner::run`), `crates/runner/src/command/sequential.rs:110` (`CommandRunner::run_raw`)

**What**: Both `# Errors` sections list "if a step cannot be built or spawned" as an `Err` condition. The bodies only return `Err` from `expand_to_plan` (unknown id, cycle, depth limit, conflicting schedule flags, bad `after`). A step that cannot be built or spawned becomes an `Ok(Vec<StepResult>)` entry with `success == false` (`exec_command` / `exec_command_raw` → `StepResult::failure`).

**Why it matters**: An embedder that trusts the docs treats `Ok` as "everything spawned" and does not check `results.iter().all(|r| r.success)` for spawn failures. API-14 requires `# Errors` to state the conditions the function actually returns `Err` for.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The # Errors sections of run and run_raw list only plan-expansion failures, and say that build/spawn failures come back as failed StepResult entries inside Ok
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.05 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
