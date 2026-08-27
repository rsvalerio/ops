---
id: TASK-1923
title: >-
  PERF-16: the documented capture budget covers only in-flight spawns —
  StepResult retains every step's full capped stdout+stderr for the whole plan
status: Triage
assignee: []
created_date: '2026-08-27 15:45'
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
- [ ] #1 the peak-RSS doc on DEFAULT_OUTPUT_BYTE_CAP states the real retention bound (driven by plan length, not OPS_MAX_PARALLEL) or the code is changed so the documented bound holds
- [ ] #2 if retention is reduced, a decision is recorded on whether StepResult.stdout/.stderr stay populated on the success path, and the public API impact of that change is noted
- [ ] #3 a test pins the chosen contract: e.g. an N-step plan of capped-output steps asserts total retained capture bytes against the documented formula
<!-- AC:END -->
