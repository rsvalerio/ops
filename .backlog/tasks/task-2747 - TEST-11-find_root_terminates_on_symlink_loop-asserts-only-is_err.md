---
id: TASK-2747
title: 'TEST-11: find_root_terminates_on_symlink_loop asserts only is_err()'
status: To Do
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - testing
dependencies: []
parent_task_id: 'TASK-2850'
modified_files:
  - extensions-rust/cargo-toml/src/tests/find_root.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions-rust/cargo-toml/src/tests/find_root.rs:find_root_terminates_on_symlink_loop'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/find_root.rs:55` (`find_root_terminates_on_symlink_loop`)

**What**: The test builds an `a -> b -> a` symlink loop and ends with a bare `assert!(result.is_err())`. Its own comment says canonicalize fails on the loop "so we get a clear error", but the test does not check which error: `canonicalize_start` maps a `NotFound` canonicalize failure to `FindWorkspaceRootError::NotFound` and anything else (ELOOP) to `CanonicalizeFailed`, and the test accepts either.

**Why it matters**: The typed-error split is a public contract (`is_not_found()` drives `is_manifest_missing` in `extensions-rust/about`). A symlink loop reported as `NotFound` would make consumers treat it as "no manifest" instead of an I/O fault, and this test would not notice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test asserts the specific FindWorkspaceRootError variant produced for a symlink loop (expected CanonicalizeFailed with the start path), not just is_err()
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.12 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
