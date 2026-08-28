---
id: TASK-1952
title: >-
  TEST-31: the --detailed-exitcode contract is implemented but never asserted by
  any test
status: To Do
assignee:
  - TASK-2002
created_date: '2026-08-27 15:49'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - testing
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:172-177`, tests at `:527-561`

**What**: The pipeline's externally observable contract is its exit code:

    let code = if opts.detailed_exitcode && changes_present { 2u8 } else { 0u8 };
    Ok(ExitCode::from(code))

`crates/cli/src/main.rs:291` returns this straight out of `main`, so it is the process exit status that CI pipelines branch on. No test asserts it. The one test that gets close, `run_plan_pipeline_to_writes_to_supplied_buffer` at `:548`, discards it: `let _code = run_plan_pipeline_to(&opts, &mut buf)...`. Nothing constructs `PlanOptions` with `detailed_exitcode: true` anywhere in the crate.

Also uncovered at the process level: `--show-outputs` (the `:159-166` branch and `render_outputs_table` are never reached from a pipeline test), `--keep-plan`, the `-` stdin form of `--json-file`, and the `bail!("plan JSON is empty")` guard at `:144-146`.

**Why it matters**: TEST-31 - a CLI's real interface is the binary: flags, stdout/stderr routing and exit codes. A silent flip of 2 to 0 here would let a CI gate report "no changes" for a plan that has changes, and no test in the workspace would fail. `ExitCode` is opaque and not comparable, so the assertion needs either a small internal function returning the `u8` (testable directly) or an `assert_cmd`-driven process test.

**Note**: `ExitCode` does not implement `PartialEq`, which is presumably why this was never asserted - splitting the code computation into a returnable value is the cheap fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test asserts the pipeline yields exit code 2 for detailed_exitcode with changes present
- [ ] #2 A test asserts it yields 0 for detailed_exitcode with a no-op-only plan, and 0 without detailed_exitcode when changes are present
- [ ] #3 A test exercises --show-outputs end to end and asserts the outputs table appears in the sink
- [ ] #4 A test asserts the empty-plan-JSON guard produces an error naming the empty input
<!-- AC:END -->
