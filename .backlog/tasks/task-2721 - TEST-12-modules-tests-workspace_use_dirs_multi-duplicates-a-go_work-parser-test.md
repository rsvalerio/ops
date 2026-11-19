---
id: TASK-2721
title: 'TEST-12: modules::tests::workspace_use_dirs_multi duplicates a go_work parser test'
status: To Do
assignee: []
created_date: '2026-11-18 18:33'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/modules.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-10'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:209`

**What**: `workspace_use_dirs_multi` in the `modules` test module calls only `crate::go_work::parse_use_dirs` on a `use (./api ./cmd)` fixture and asserts the raw list — the same path `go_work::tests::parses_multi_use_block` (go_work.rs:146) and `block_opener_accepts_no_space_before_paren` already cover. It exercises no `modules` code.

**Why it matters**: A redundant test in the wrong module adds maintenance without coverage, and suggests `modules` has its own parsing behaviour when it does not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 workspace_use_dirs_multi is removed (or rewritten to assert modules-level behaviour not covered by collect_units_workspace)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
