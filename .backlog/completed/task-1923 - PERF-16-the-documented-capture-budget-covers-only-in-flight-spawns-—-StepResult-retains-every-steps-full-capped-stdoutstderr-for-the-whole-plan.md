---
id: TASK-1923
title: >-
  PERF-16: the documented capture budget covers only in-flight spawns —
  StepResult retains every step's full capped stdout+stderr for the whole plan
status: Done
assignee:
  - TASK-1986
created_date: '2026-08-27 15:45'
updated_date: '2026-08-28 19:08'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - crates/runner/src/command/results.rs
  - crates/runner/src/command/parallel.rs
  - crates/runner/src/command/sequential.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:117-128` (the peak-RSS budget doc on `DEFAULT_OUTPUT_BYTE_CAP`), `crates/runner/src/command/results.rs:11-18` (`StepResult.stdout` / `.stderr`), `crates/runner/src/command/parallel.rs:478` (`collect_join_results_with_pre`), `crates/runner/src/command/sequential.rs:706-717` (`run_plan` results vector)

**What**: the doc block on `DEFAULT_OUTPUT_BYTE_CAP` states the memory contract:

> This cap applies *per spawn × per stream*, so the worst-case in-flight capture budget is `OPS_MAX_PARALLEL × 2 × cap`. With the defaults (`OPS_MAX_PARALLEL=32`, `cap=4 MiB`) that's ≤ 256 MiB

That bound is real for *in-flight* buffers, but it is not the runner's peak. `build_step_result` (`exec.rs:346-359`) moves the capped `stdout` and `stderr` `String`s into the returned `StepResult`, and both `run_plan` and `run_plan_parallel` accumulate one `StepResult` per step into a `Vec` that lives until the whole plan finishes and the caller drops it. Retention is therefore governed by the **plan length**, not by the parallel width:

`steps × 2 × cap`, not `min(steps, max_parallel) × 2 × cap`

With the defaults, a 200-step plan of noisy steps retains up to 1.6 GiB — six times the number the doc tells an operator to budget for, and unaffected by the `OPS_MAX_PARALLEL` knob the doc tells them to dial down. Lowering `OPS_MAX_PARALLEL` to 1 does not reduce it at all. Nothing else caps it: `output_byte_cap` is per stream, and there is no plan-level ceiling on the results vector.

This matters more than a stale doc because the doc is the operator's tuning guidance ("Operators tuning the cap on tight CI runners should also dial down `OPS_MAX_PARALLEL` accordingly") and following it does not bound the thing it claims to bound. The two honest fixes are (a) correct the doc to `steps × 2 × cap` and give operators a knob that actually governs it, or (b) stop retaining full captures in `StepResult` for steps whose output has already been emitted as `StepOutput` events and written to the tap — the successful-step case has no remaining consumer for the buffer.

Worth checking as part of the fix: `StepResult.stdout` / `.stderr` are `pub` on a `#[non_exhaustive]` struct, so trimming retention is an observable API change and needs a decision on whether success-path captures are part of the contract.

**Why it matters**: a long `.ops.toml` plan on a memory-constrained CI runner is exactly the configuration this cap was introduced (PERF-1 / TASK-0515 / TASK-0764) to protect, and it is the configuration where the published budget is most wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 the peak-RSS doc on DEFAULT_OUTPUT_BYTE_CAP states the real retention bound (driven by plan length, not OPS_MAX_PARALLEL) or the code is changed so the documented bound holds
- [x] #2 if retention is reduced, a decision is recorded on whether StepResult.stdout/.stderr stay populated on the success path, and the public API impact of that change is noted
- [x] #3 a test pins the chosen contract: e.g. an N-step plan of capped-output steps asserts total retained capture bytes against the documented formula
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Chose the doc-correction branch of AC #1: the DEFAULT_OUTPUT_BYTE_CAP doc now separates the in-flight budget (OPS_MAX_PARALLEL x 2 x cap) from retention (steps x 2 x cap), states explicitly that lowering OPS_MAX_PARALLEL does not reduce retention, and names the knobs that do (OPS_OUTPUT_BYTE_CAP, plan length). PEAK_CAPTURE_WARN_BYTES doc now says its warning covers the in-flight budget only. AC #2 decision recorded in the same doc block: StepResult.stdout/.stderr stay populated on the success path — they are pub on a #[non_exhaustive] struct and part of the shape embedders read, so emptying them would be an unannounced behavioural change; only the in-tree consumer (log_step_results) reads their length. Test retained_capture_bytes_scale_with_plan_length runs an 8-step *sequential* plan (one step in flight) and asserts retained bytes == steps x line_len, i.e. linear in plan length, plus the documented bound.
<!-- SECTION:NOTES:END -->
