---
id: TASK-1224
title: >-
  ERR-1: Variables::expand swallows ExpandError to tracing::warn losing
  user-facing diagnostic
status: To Do
assignee:
  - TASK-1268
created_date: '2026-05-08 12:57'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:117-129`

**What**: `expand()` falls back to `Cow::Borrowed(input)` on any `try_expand` error, emitting only a tracing::warn. Strict callers go through `try_expand` (good), but every dry-run / display-only path silently renders the literal `\${VAR}` while the warning sits at warn level — filtered by the default OPS_LOG_LEVEL.

**Why it matters**: Users debugging "why does my dry-run show \${HOME} literally" must know to set OPS_LOG_LEVEL=warn. The contract should surface a rate-limited ui::warn so the visible output and strict-path failure correlate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Surface first per-key failure via crate::ui::warn (rate-limited via OnceLock<HashSet>)
- [ ] #2 Keep tracing::warn for structured logs
- [ ] #3 Test asserts ui::warn fires once per distinct var
<!-- AC:END -->
