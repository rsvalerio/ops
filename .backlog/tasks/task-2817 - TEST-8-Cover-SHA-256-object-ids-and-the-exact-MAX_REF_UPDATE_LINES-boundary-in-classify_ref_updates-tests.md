---
id: TASK-2817
title: 'TEST-8: Cover SHA-256 object ids and the exact MAX_REF_UPDATE_LINES boundary in classify_ref_updates tests'
status: Done
assignee: []
created_date: '2026-11-18 18:47'
updated_date: '2026-11-18 23:00'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2862'
modified_files:
  - extensions/run-before-push/src/lib.rs
  - extensions/run-before-push/tests/classify_ref_updates.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-8:extensions/run-before-push/src/lib.rs:classify_ref_updates'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:198-200` (`is_object_id`), `:230` (line-bound check in `classify_ref_updates`), `:657-662` (only line-bound test); `extensions/run-before-push/tests/classify_ref_updates.rs` (all cases use 40-hex oids)

**What**: Every classifier test uses 40-hex SHA-1 oids; the 64-hex SHA-256 arm of `is_object_id` (`matches!(field.len(), 40 | 64)`) is never exercised, so narrowing it to `40` (a mutant) passes the suite. The line bound is tested only at `MAX_REF_UPDATE_LINES + 1` (=> Run); there is no test that a stream of exactly `MAX_REF_UPDATE_LINES` well-formed deletions still classifies as `DeleteOnly`, so an off-by-one tightening of the bound goes unnoticed. There is also no case for a 64-hex all-zero deletion oid.

**Why it matters**: SHA-256 repositories are supported by git; a regression in that arm would silently force `Run` (or worse, mis-shape validation) only for those users. Boundary values are where the bound logic breaks (TEST-8).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tests classify a SHA-256 update line as Run and a SHA-256 (64 zero) deletion line as DeleteOnly
- [x] #2 A test asserts a stream of exactly MAX_REF_UPDATE_LINES deletions classifies as DeleteOnly, alongside the existing +1 => Run test

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.29 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
