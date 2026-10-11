---
id: TASK-2527
title: 'TEST-11: coverage_collect_fails_with_nonexistent_directory asserts only is_err()'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:51'
labels:
  - code-review
  - test
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions-rust/test-coverage/src/ingestor.rs:coverage_collect_fails_with_nonexistent_directory'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/ingestor.rs:52-53`

**What**: `coverage_collect_fails_with_nonexistent_directory` ends with `assert!(result.is_err());` and inspects nothing else — the only assertion-only-on-`is_err()` site in the crate's suite (the sibling tests in `tests/wiring.rs` and `tests/parse_edge.rs` all `unwrap_err()` and check the message content). The test cannot distinguish the expected collect failure from an unrelated error (e.g. a fixture-staging failure), so it can pass for the wrong reason.

**Why it matters**: TEST-11: assert specific values, not just `is_ok()`/`is_err()`. Asserting the error names the missing working directory (it flows into the cargo invocation / collect context) would pin that the failure is the intended one.

<!-- scan confidence: candidates to inspect — single site, verified at source -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test asserts on the error's content (e.g. the rendered error names the nonexistent working directory or the failing collect step), not only result.is_err()

<!-- AC:END -->
