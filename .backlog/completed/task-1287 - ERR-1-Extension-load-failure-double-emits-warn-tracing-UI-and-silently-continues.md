---
id: TASK-1287
title: >-
  ERR-1: Extension load failure double-emits warn (tracing + UI) and silently
  continues
status: Done
assignee:
  - TASK-1304
created_date: '2026-05-11 15:27'
updated_date: '2026-05-11 18:06'
labels:
  - code-review-rust
  - error
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:58-68`

**What**: When `builtin_extensions` errors during hook install, the code emits both a `tracing::warn!` and an `ops_core::ui::warn` carrying nearly the same message, then silently continues with an empty extension registry. ERR-1 cautions against handling an error and also logging it (or double-logging across two channels).

**Why it matters**: Operators get two warnings for one fault and the install proceeds with a degraded command list as if nothing happened — a misconfigured extension config silently degrades selection rather than surfacing as an error the user can fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit the diagnostic through exactly one channel (UI for the operator, or tracing with structured fields — not both with similar prose)
- [ ] #2 Document the degradation policy (continue with built-in commands only) in a comment
- [ ] #3 Consider whether bailing is more appropriate when the user has extensions.enabled set — they likely want hard-fail on misconfiguration
<!-- AC:END -->
