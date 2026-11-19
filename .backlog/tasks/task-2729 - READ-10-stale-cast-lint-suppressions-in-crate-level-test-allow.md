---
id: TASK-2729
title: 'READ-10: stale cast-lint suppressions in crate-level test allow'
status: To Do
assignee: []
created_date: '2026-11-18 18:35'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-node/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-10:extensions-node/about/src/lib.rs:crate-attributes'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:11`

**What**: `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss))]` suppresses three cast lints crate-wide under test. The crate has no `as` numeric casts in any module, test modules included, so those three allows suppress nothing. They are bare `allow`s with no reason, so they will never report themselves as stale. The sibling `extensions-python/about/src/lib.rs:15` allows only `clippy::unwrap_used`.

**Why it matters**: A blanket suppression with no live trigger hides any cast added later to this crate's tests, and it signals a policy exception that does not exist.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three clippy::cast_* entries are removed from the crate-level cfg_attr(test, allow(...))
- [ ] #2 cargo clippy --all-targets -p ops-about-node stays clean
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.09 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
