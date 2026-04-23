---
id: TASK-0220
title: 'ERR-4: db.lock().ok()? discards PoisonError context in about code page'
status: To Do
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:31`

**What**: `let conn = db.lock().ok()?;` converts any lock poisoning into silent None so the subpage shows empty output with no log.

**Why it matters**: Hides real failures (poisoned lock after a panic) from users and operators.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log the PoisonError with tracing::warn before returning None
- [ ] #2 Add regression test asserting a warning is emitted on poisoned lock
<!-- AC:END -->
