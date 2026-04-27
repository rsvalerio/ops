---
id: TASK-0408
title: >-
  ERR-1: parallel cancellation paths build StepResult::skipped which is
  success=true and indistinguishable from a clean skip
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:53'
updated_date: '2026-04-26 10:22'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:39-42` (`StepResult::skipped`); call sites: `crates/runner/src/command/parallel.rs:61` (`fail_fast` cancellation in `collect_join_results`), `crates/runner/src/command/exec.rs:301` (`exec_standalone` early-return when abort flag is already set)

**What**: `StepResult::skipped` is implemented as `Self::new(id, true, Duration::ZERO)` — `success: true`. Two production paths use it for *cancellation*, not for an explicit user skip:

1. `parallel.rs::collect_join_results` converts a `JoinError::is_cancelled` (a sibling task aborted by `fail_fast`) into `StepResult::skipped("<cancelled>")`.
2. `exec.rs::exec_standalone` checks the abort flag at entry and returns `StepResult::skipped(id)` if it is already set.

Both aggregate later via `results.iter().all(|r| r.success)` (sequential.rs:50, parallel.rs:178) — meaning a cancelled task is counted as a successful step in the plan-success calculation. The original failure that triggered the cancellation is in the same vector with `success: false`, so the overall `all(...)` still returns `false` and the user-visible exit code is correct *today*. But the invariant relies on "the failing task is always in the same Vec", which is fragile: any future refactor that filters or buffers results by lifecycle phase will silently consider cancelled steps successful.

**Why it matters**: The struct contract for `success: true` is "this step succeeded". Encoding "this step never ran because we cancelled it" with `success = true` overloads that field. Downstream consumers (telemetry, exit code, CI summary) cannot distinguish a clean skip-by-config from an aborted-by-failure. A separate `StepResult::cancelled` constructor with `success: false` (or a typed status enum) would make the distinction load-bearing in the type rather than in caller convention.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cancellation paths in parallel.rs and exec.rs no longer use a constructor named skipped(); they use a distinct constructor (e.g. StepResult::cancelled) that records cancellation as success=false (or a typed enum status field)
- [ ] #2 Plan-success aggregation still produces the same exit code, verified by an existing or new test for fail_fast cancellation
- [ ] #3 Display layer treats cancelled differently from skipped (or chooses to render them identically with a deliberate comment)
<!-- AC:END -->
