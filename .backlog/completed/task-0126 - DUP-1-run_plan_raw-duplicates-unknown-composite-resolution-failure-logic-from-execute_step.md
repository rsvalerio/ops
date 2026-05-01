---
id: TASK-0126
title: >-
  DUP-1: run_plan_raw duplicates unknown/composite resolution-failure logic from
  execute_step
status: Done
assignee: []
created_date: '2026-04-20 20:21'
updated_date: '2026-04-20 20:45'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:380-406` (`run_plan_raw`) vs `crates/runner/src/command/mod.rs:321-341` (`execute_step`)

**What**: `run_plan_raw` re-implements the resolve + unknown + composite-in-leaf-plan error cases inline, producing the *same* user-facing messages as `execute_step` ("unknown command: {}", "internal error: composite in leaf plan: {}") but via a parallel code structure. `execute_step` routes through the shared `resolution_failure()` helper that also emits `RunnerEvent::StepFailed`; `run_plan_raw` pushes `StepResult::failure` directly and intentionally omits the event. The two paths now diverge in how they surface the same condition — any future wording or payload change has to be made twice.

**Why it matters**: Duplicated resolution logic in the runner is a drift source. The messages already agree today only because both were copy-edited at the same time. Behavioural parity (same text, same `Duration::ZERO`, same `fail_fast` semantics) is contract — it belongs in a helper.

**Suggested shape**: extract something like `fn resolve_exec_leaf(&self, id: &str) -> Result<ExecCommandSpec, String>` returning `Err(message)` for both unknown and composite-in-leaf-plan cases; let `execute_step` keep emitting the event on top of the shared result and `run_plan_raw` just push the failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_plan_raw and execute_step share one source of truth for resolving a leaf ID to an ExecCommandSpec with unified error messages
- [ ] #2 The 'unknown command' and 'internal error: composite in leaf plan' strings are constructed in exactly one place
- [ ] #3 Existing raw and non-raw resolution-failure tests still pass
<!-- AC:END -->
