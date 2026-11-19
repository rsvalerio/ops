---
id: TASK-2719
title: 'TEST-11: nested use( opener test neither asserts the warn nor the entry after it'
status: Done
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 22:20'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/go_work.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-8'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:224`

**What**: `parse_use_dirs_warns_on_nested_block_opener` is named for the warning and its doc says "the real entries around it still resolve", but it asserts neither: there is no `count_warnings`/`capture_warn` check that the `tracing::warn!` at go_work.rs:63 fired, and only `./api` (before the nested opener) is checked — `./cmd` (after it) is not. The negative assertion `!d.contains("use (") || d == "use("` is also weaker than asserting the exact list.

**Why it matters**: Deleting the warn, or a regression that drops entries following the nested opener, would leave this test green while contradicting its name and doc (TEST-2/TEST-11).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test asserts exactly one warn via ops_core::test_utils::count_warnings
- [x] #2 The test asserts the exact resulting list (assert_eq!(dirs, vec!["./api", "./cmd"]))

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
