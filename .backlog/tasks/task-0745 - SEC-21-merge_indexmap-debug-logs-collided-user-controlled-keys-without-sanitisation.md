---
id: TASK-0745
title: >-
  SEC-21: merge_indexmap debug-logs collided user-controlled keys without
  sanitisation
status: To Do
assignee:
  - TASK-0827
created_date: '2026-05-01 05:52'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:23-29`

**What**: When an overlay shadows base entries (commands, themes, tools), the function emits `tracing::debug!(keys = ?replaced, ...)` with Debug formatting on user-controlled config keys. The keys may contain newlines/control bytes; downstream subscribers rendering with Display (or piping into a flat-line log file) will not preserve the Debug escaping.

**Why it matters**: Same class as TASK-0665 (path Display log fields). A config with `[commands."foo\n2026-01-01 ERROR injected"]` can forge structured-log entries when keys collide.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sanitize keys before logging (escape control chars/newlines) or restrict the logged form to a stable identifier subset
- [ ] #2 Regression test injects a control-character-bearing key and asserts the emitted log line cannot forge an entry under a Display subscriber
<!-- AC:END -->
