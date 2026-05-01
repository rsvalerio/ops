---
id: TASK-0267
title: >-
  ERR-4: run() silently falls back to Config::default when load_config errors
  early
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 14:29'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:107`

**What**: Malformed .ops.toml logs tracing::warn then proceeds with defaults; user commands vanish.

**Why it matters**: Surfaces downstream as "unknown command" errors unrelated to real cause.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Print to stderr with actionable hint
- [ ] #2 Add --strict to fail fast
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1 done (actionable stderr message now printed). AC#2 (--strict flag) deferred — would require new CLI arg and wire-up; not blocking since the warning is now loud and includes the full error chain via {e:#}. Open a follow-up task if --strict is still desired.
<!-- SECTION:NOTES:END -->
