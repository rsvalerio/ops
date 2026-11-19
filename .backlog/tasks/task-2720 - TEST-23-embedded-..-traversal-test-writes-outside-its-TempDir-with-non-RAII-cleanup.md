---
id: TASK-2720
title: 'TEST-23: embedded-.. traversal test writes outside its TempDir with non-RAII cleanup'
status: Done
assignee: []
created_date: '2026-11-18 18:33'
updated_date: '2026-11-18 22:20'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/modules.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-9'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:424`

**What**: `collect_units_embedded_parent_dir_use_directive_is_marked_out_of_tree` creates `<tempdir parent>/<leaf>-sec14` — a sibling of its `TempDir` in the shared system temp directory — and removes it with `std::fs::remove_dir_all(&target).ok()` placed after the call under test. If `collect_units` panics (or the test is killed) the directory leaks into `/tmp`. The inline comment at modules.rs:435-437 ("Keep the escape inside the tempdir's parent by targeting the tempdir itself via the traversal, then a nested marker dir") does not describe what the code does. The neighbouring test `collect_units_leading_parent_dir_use_directive_still_reads_go_mod` (modules.rs:473) already shows the isolated shape: one `TempDir` root with `project/` and a sibling inside it.

**Why it matters**: Tests should own all filesystem state they create (TEST-19/TEST-23); leaked fixtures accumulate in shared temp and the misleading comment obscures a security-relevant test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test uses a single TempDir root with cwd = root/project and the traversal target inside root, so all state is removed by TempDir's Drop
- [x] #2 The manual remove_dir_all and the inaccurate comment are gone

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
