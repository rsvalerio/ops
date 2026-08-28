---
id: TASK-1958
title: 'FN-1: run_terraform_pipeline is an 80-line function doing six unrelated jobs'
status: To Do
assignee:
  - TASK-2002
created_date: '2026-08-27 15:50'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - complexity
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:272-351`

**What**: 80 lines in one function, covering:

1. path expansion for two artefact paths (`:273-274`)
2. directory creation for both (`:276-281`)
3. building and configuring the `terraform plan` command (`:283-298`)
4. two different exit-status interpretations depending on `detailed_exitcode`, with the `bail!` body duplicated at `:317-320` and `:324-327` (`status.code().unwrap_or(1)` in both)
5. a second subprocess (`terraform show -json`), its failure handling, and UTF-8 conversion (`:330-344`)
6. conditionally persisting the JSON artefact (`:346-348`)

The duplicated `bail!` in step 4 is the visible symptom: the two branches differ only in which codes count as success, so the shared failure message got copy-pasted.

**Why it matters**: FN-1 caps functions at 50 lines. Beyond length, the mixing is what blocks testing - there is no seam to inject a fake command runner, which is why every other finding on this function (the uncapped `output()` under SEC-33, the raw stderr under SEC-21, the pathless `?` under ERR-13, the exit-code contract under TEST-31) has no test attached to it today.

**Suggested fix**: split into `prepare_artifact_paths`, `run_terraform_plan(opts, &binary_path) -> Result<()>` (owning both exit-status branches with one shared failure constructor), and `capture_plan_json(&binary_path) -> Result<String>`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_terraform_pipeline is under 50 lines and delegates path preparation, plan invocation and JSON capture to named helpers
- [ ] #2 The two exit-status branches share one failure-message constructor instead of duplicating the bail body
- [ ] #3 Existing behaviour for detailed_exitcode 0 and 2, and for non-detailed success, is unchanged
<!-- AC:END -->
