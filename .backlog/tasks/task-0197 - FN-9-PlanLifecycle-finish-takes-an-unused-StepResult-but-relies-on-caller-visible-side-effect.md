---
id: TASK-0197
title: >-
  FN-9: PlanLifecycle::finish takes an unused &[StepResult] but relies on
  caller-visible side effect
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:67-73` (PlanLifecycle::finish).

**What**: `fn finish(self, results: &[StepResult], on_event: &mut impl FnMut(RunnerEvent))` computes `let success = results.iter().all(|r| r.success);` and emits `RunFinished`. The `results` slice is only inspected to derive a bool. Two design nits: (1) the function could take just `success: bool` and let callers decide how to compute it — today every caller constructs the same "all successes" predicate inline at the callsite after already having inspected results individually in the loop; (2) the function mixes lifecycle timing (captured `Instant`) with aggregation logic, fighting FN-9 (explicit dependencies).

**Why it matters**: FN-9 / API-1 nit. Not a correctness issue, but the current signature makes it easy for future callers to pass a `results` slice that does not actually reflect the run outcome (e.g. partial results after an early-return). Fix: take `fn finish(self, success: bool, on_event: ...)` and let callers compute success explicitly. Minor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change PlanLifecycle::finish signature to take success: bool explicitly
<!-- AC:END -->
