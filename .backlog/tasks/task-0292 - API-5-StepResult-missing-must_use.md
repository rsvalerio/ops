---
id: TASK-0292
title: 'API-5: StepResult missing #[must_use]'
status: Done
assignee:
  - TASK-0298
created_date: '2026-04-23 16:54'
updated_date: '2026-04-23 17:17'
labels:
  - rust-code-review
  - api
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:8`

**What**: `pub struct StepResult` is returned by the core runner execution paths (`run_exec`, `run_plan`, `run_plan_parallel`) but is not annotated `#[must_use]`. Callers that accidentally discard the value silently drop success/failure status, stderr, and timing.

**Why it matters**: API-5 mandates `#[must_use]` on Results/Futures/status-carrying types. Unused results are easy to introduce during refactors of CLI/runner code and hide command failures until much later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 StepResult is annotated #[must_use] (with a reason string if preferred)
- [x] #2 cargo build --workspace --all-targets still passes
- [x] #3 clippy runs clean; no existing callsite was silently discarding StepResult
<!-- AC:END -->
