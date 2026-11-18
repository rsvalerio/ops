---
id: TASK-2768
title: 'SEC-31: classify_plan silently drops resource changes with an empty actions array'
status: Done
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 22:53'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/plan/src/lib.rs
  - extensions-terraform/plan/src/model.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-31:extensions-terraform/plan/src/lib.rs:classify_plan'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:258` (`classify_plan`), `extensions-terraform/plan/src/model.rs:103` (`Action::classify`)

**What**: `Action::classify` returns `None` for an empty `actions` list, and `classify_plan` uses `filter_map`, so a `resource_changes` entry whose `change.actions` is `[]` vanishes. It is missing from the summary table, the resource table and the `Plan:` line, and it does not count toward `has_changes`, so the `--detailed-exitcode` gate can report 0. The outputs side does the opposite on purpose: `classify_output_action` (`render.rs:242`) maps an empty `actions` list to `Action::Unknown` with a warning. Its doc says reporting "nothing" is wrong "precisely when this build could not tell".

**Why it matters**: The plan JSON is untrusted input (a wrapped `terraform` on PATH, a hand-supplied `--json-file`, a future format change). A resource row the tool cannot read should fail closed, with a visible `unknown` row, the warning banner and a non-zero detailed exit code. Today it fails open and disappears from the screen an operator approves an apply from. The two halves of the crate also disagree on the same input shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A resource change with an empty actions array surfaces as Action::Unknown (with a tracing::warn!), matching classify_output_action
- [x] #2 Such a row appears in the resource table with the unknown banner and makes has_changes return true
- [x] #3 A unit test covers a resource_changes entry with actions: []

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.21 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
