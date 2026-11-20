---
id: TASK-2757
title: 'TEST-11: signal-kill-with-partial-stderr deny test asserts only is_err()'
status: Done
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 22:41'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2851'
modified_files:
  - extensions-rust/deps/src/parse/deny/tests.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions-rust/deps/src/parse/deny/tests.rs:interpret_deny_result_errs_on_signal_kill_even_with_partial_stderr'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny/tests.rs:151`

**What**: `interpret_deny_result_errs_on_signal_kill_even_with_partial_stderr` feeds `interpret_deny_result(None, <valid vulnerability diagnostic>)` and asserts only `result.is_err()`. Its sibling `interpret_deny_result_errs_on_signal_kill` (line 144) checks that the error names the signal case: `err.to_string().contains("signal")`.

**Why it matters**: The test exists to pin one property: a signal exit is rejected *even though* the stderr carries a decodable finding. If the `None` arm were reordered behind a parse path that failed for an unrelated reason, `is_err()` would still pass. The test would then stop checking the signal-kill guard without anyone noticing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test unwraps the error with expect_err and asserts that its message names the signal-kill case, as its sibling test does
- [x] #2 The ops-deps tests pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.15 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
