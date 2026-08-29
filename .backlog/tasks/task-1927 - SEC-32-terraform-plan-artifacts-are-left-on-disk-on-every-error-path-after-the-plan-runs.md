---
id: TASK-1927
title: >-
  SEC-32: terraform plan artifacts are left on disk on every error path after
  the plan runs
status: Done
assignee:
  - TASK-2002
created_date: '2026-08-27 15:46'
updated_date: '2026-08-28 21:30'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:131-178` (`run_plan_pipeline_to_with_tty`), cleanup at `:168-170`

**What**: `cleanup_artifacts(opts)` is only reached on the success path. Every early exit after `run_terraform_pipeline` has already produced `.ops/tfplan.binary` returns without deleting it:

- `bail!("plan JSON is empty")` at `:144-146`
- `parse_and_classify(&json_str)?` at `:148`
- `write!(out, ...).context("write summary table")?` at `:151`, and the resource/outputs writes at `:156` and `:162` (a closed pipe, e.g. `ops plans | head`, is enough)

The binary plan file is the full planned state: provider blocks with embedded credentials, generated passwords, and sensitive output values. With `--keep-plan` the same holds for the JSON copy written at `:347`. `.ops/` is not listed in the repo `.gitignore`, so the residue can also be committed by accident.

**Why it matters**: A transient rendering or parse failure silently converts a short-lived temporary into a persistent secret-bearing file that the user believes was cleaned up (the tool deletes it on the happy path, so nobody looks). SEC-32 requires sensitive resources to be cleaned up on error paths, not just on success.

**Suggested fix**: run cleanup from a guard/`Drop` or a single wrapper that runs it on both `Ok` and `Err` (keeping the existing `!keep_plan && json_file.is_none()` condition), rather than as a statement on the success path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cleanup_artifacts runs on the error paths of run_plan_pipeline_to_with_tty as well as the success path, under the same !keep_plan && json_file.is_none() condition
- [x] #2 A test asserts that a plan artifact staged before an induced pipeline failure (empty JSON or unparseable JSON) is removed when keep_plan is false
- [x] #3 A test asserts the artifact survives the same failure when keep_plan is true
<!-- AC:END -->
