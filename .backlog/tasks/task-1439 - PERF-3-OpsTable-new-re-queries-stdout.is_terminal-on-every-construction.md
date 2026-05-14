---
id: TASK-1439
title: 'PERF-3: OpsTable::new re-queries stdout().is_terminal() on every construction'
status: Done
assignee:
  - TASK-1459
created_date: '2026-05-13 18:40'
updated_date: '2026-05-14 08:35'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/table.rs:27-28`

**What**: `OpsTable::new()` calls `std::io::stdout().is_terminal()` on every invocation. `theme_cmd`/`tools_cmd`/`help` and other rendering paths build tables on each invocation and retry, paying the `isatty` syscall each time.

**Why it matters**: `style::color_enabled` already memoises `is_terminal()` per stream via `OnceLock`. The two probes can disagree mid-render after a redirect, and the duplicate syscall is avoidable. Quality / minor perf, but a clear single-source-of-truth violation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OpsTable::new routes the TTY probe through a shared OnceLock<bool> (or style::color_enabled adjacent helper) so repeated calls do not re-invoke is_terminal
- [ ] #2 Test asserts only one is_terminal probe is observed across N table constructions in the same process
<!-- AC:END -->
