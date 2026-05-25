---
id: TASK-1225
title: >-
  DUP-3: FromIterator for CommandRegistry silently drops the duplicate-insert
  audit trail
status: Done
assignee:
  - TASK-1265
created_date: '2026-05-08 12:57'
updated_date: '2026-05-09 13:49'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:192-200`

**What**: The `FromIterator<(CommandId, CommandSpec)>` impl calls `reg.insert(...)` which records duplicates into `duplicate_inserts`, but the resulting registry is returned by value with no API hint that callers should call `take_duplicate_inserts`. Code building registries via `collect()`/`from_iter()` silently loses the warning signal that the per-extension registration path explicitly preserves.

**Why it matters**: ERR-2 / TASK-0579 hardened the .insert() path to preserve duplicates; the FromIterator path bypasses that contract entirely if callers aren't aware. Duplicates from collect() are observed only by inspection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Drain in FromIterator and emit tracing::warn from there
- [ ] #2 OR document loss of audit and redirect callers to insert()
- [x] #3 Add a doc-test or assertion in tests
<!-- AC:END -->
