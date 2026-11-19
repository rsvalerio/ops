---
id: TASK-2769
title: 'PATTERN-1: PlanOptions silently ignores terraform-mode flags under --json-file'
status: To Do
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'PATTERN-1:extensions-terraform/plan/src/lib.rs:PlanOptions'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:50` (`PlanOptions`), consumed in `plan_pipeline_body` (`lib.rs:205`) and `with_artifact_cleanup` (`lib.rs:252`)

**What**: `json_file: Option<String>` decides which of two modes the run is in, and four sibling fields only mean something in one of them: `passthrough`, `out`, `json_out` and `keep_plan` apply only when `json_file` is `None`. No clap `conflicts_with` relates them, so `ops plans --json-file plan.json --keep-plan --out x -- -target=foo` is accepted and every terraform-mode flag is dropped without a word. The `passthrough` doc says "(default mode only)", but nothing enforces that.

**Why it matters**: This is the PATTERN-1 "field beside the data it gates" shape: combinations that mean nothing are representable and reachable from the command line. An operator who passes `-target=...` or `--keep-plan` alongside `--json-file` believes the run was narrowed or the artifacts kept, and gets no signal that it was not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Passing --out, --json-out, --keep-plan or passthrough args together with --json-file is rejected at parse time (clap conflicts_with / ArgGroup) or modelled as a mode enum
- [ ] #2 A test asserts the rejected combinations produce a clear error
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.21 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
