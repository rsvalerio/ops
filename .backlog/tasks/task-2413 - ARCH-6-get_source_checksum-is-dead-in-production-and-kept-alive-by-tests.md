---
id: TASK-2413
title: 'ARCH-6: get_source_checksum is dead in production and kept alive by tests'
status: To Do
assignee: []
created_date: '2026-10-04 14:18'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - ARCH
dependencies: []
parent_task_id: 'TASK-2416'
modified_files:
  - extensions/sqlite/src/schema.rs
priority: low
ordinal: 1000
dedup_key: 'ARCH-6:extensions/sqlite/src/schema.rs:get_source_checksum'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/schema.rs:36-72` (`get_source_checksum`)

**What**: The function lives in a private, non-re-exported module and has no production caller (confirmed by its own `#[cfg_attr(not(test), expect(dead_code, ...))]`; no use elsewhere in the repo). It carries its own `#[must_use]` message and API-5 rationale and is exercised only by the crate's tests.

**Why it matters**: It is speculative surface kept compiling and documented for nothing. Delete it with its tests, or wire it into the reload-check path it was written for.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 get_source_checksum and its dead_code expect are removed, or a production caller uses it
<!-- AC:END -->
