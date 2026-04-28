---
id: TASK-0452
title: 'SEC-25: atomic_write does not log cleanup failure when rename fails'
status: To Do
assignee:
  - TASK-0538
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:106-118`

**What**: When `rename` fails, `atomic_write` calls `remove_file(&tmp)` (best-effort) and returns the rename error, but the cleanup result is discarded with `let _`. A rename-then-remove failure leaves a stray `.foo.toml.tmp.<pid>.<n>.<nanos>` next to the user file with no trace.

**Why it matters**: Repeated failures (full disk, perms) accumulate clutter; matches the "let _ on cleanup" anti-pattern that hides resource issues.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log at tracing::warn! if cleanup fails — a leaked tmp file is worth a single line
- [ ] #2 Test asserts no leftover .tmp.* files after a forced rename failure (or that we logged when one is left behind)
<!-- AC:END -->
