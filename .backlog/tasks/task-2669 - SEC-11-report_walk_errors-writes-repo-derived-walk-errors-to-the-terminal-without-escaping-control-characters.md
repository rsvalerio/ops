---
id: TASK-2669
title: 'SEC-11: report_walk_errors writes repo-derived walk errors to the terminal without escaping control characters'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2841'
modified_files:
  - crates/core/src/bounded_read.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-11:crates/core/src/bounded_read.rs:report_walk_errors'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/bounded_read.rs:309-312` (`report_walk_errors`)

**What**: `record_failure` (same file, line ~286) deliberately routes both the path and the message through `safe_line_text`, because "both the path and the message originate in the walked tree (an extension root can be repo-supplied)". `report_walk_errors` writes each discovery walk error with a bare `writeln!(writer, "{label}: walk error: {error}")`. Those strings are rendered `ignore`/walkdir errors and embed the offending path (e.g. `IO error for operation on <path>: permission denied`, see `extensions/text-fixers/src/report.rs` test data), so a hostile filename containing `\n`, ESC/CSI sequences, or bidi controls reaches the operator's terminal raw.

**Why it matters**: The same repo-controlled path that `record_failure` neutralises can forge extra report lines (e.g. a fake `ok` summary) or repaint the terminal when it surfaces as a walk error instead of a per-file failure — the two sibling functions in this module apply different trust policies to the same input.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 report_walk_errors renders each walk error through the same escaping helper record_failure uses
- [ ] #2 The report still adopts the raw (unescaped) error strings for programmatic consumers
- [ ] #3 A unit test feeds a walk error containing a newline and an ESC sequence and asserts the written line contains only escaped forms
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
