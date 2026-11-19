---
id: TASK-2758
title: 'ERR-1: recover_partial_report drops the report JSON parse error without a breadcrumb'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2854'
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-1:extensions-rust/test-coverage/src/parse.rs:recover_partial_report'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:369-380`

**What**: In the soft-fail path after a non-zero `cargo llvm-cov` exit, `recover_partial_report` handles a failed *read* of the report file by emitting a `warn!` naming the path and IO error, but a report that reads fine and fails to *parse* is collapsed with `serde_json::from_slice(...).ok()` and discarded with no log at all. The function then returns `None` and the caller surfaces only the cargo exit error.

**Why it matters**: The handling site swallows the error instead of logging it there (ERR-1: handle *or* propagate, and log at the handling site). A truncated/corrupt report (TMPDIR full, cargo killed mid-write) is indistinguishable in the logs from "cargo failed before instrumenting", which is exactly the diagnosis the read-error breadcrumb was added to make possible. The two arms of the same match are inconsistent (READ-6).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A report that is read but fails to parse as JSON emits a warn naming report_path and the serde error before falling through to the cargo error
- [ ] #2 A test covers the non-zero-exit + unparseable-report path and asserts the cargo error is still the returned error
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.19 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
