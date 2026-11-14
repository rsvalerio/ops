---
id: TASK-2650
title: 'ops config check is blind to field values and to behaviour changes no field marks'
status: Done
assignee: []
created_date: '2026-11-01 00:41'
updated_date: '2026-11-01 13:41'
labels:
  - code-review-rust
  - feature
  - config
dependencies: []
parent_task_id: 'TASK-2655'
modified_files:
  - crates/core/src/config/history.rs
  - docs/configuration.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/history.rs:1`

**What**: The history records fields by the release that first accepted them. It does not record accepted values (a theme name, a `layout_kind`), and it has no row kind for a behaviour change that keeps parsing with no field to mark it.

**Why it matters**: TASK-2638's description names exactly this gap: a behaviour change that still parses passes silently, so the check can report a config as accepted by a pin that loads it differently.

**Origin**: discovered during TASK-2646 while fixing TASK-2638.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A value or behaviour newer than the target is reported by ops config check, or the decision not to track it is recorded

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Values newer than their field are now reported: VALUES rows in history.rs (output.theme = flux|studio needs 0.17.0, the release d889b1dc shipped them; classic/compact date to 0.1.0). A theme name the project's own [themes] defines is suppressed - it resolves wherever [themes] does. layout_kind = boxed needs no value row: the field itself is a 0.17.0 HISTORY row that dominates. Behaviour changes with no field or value marker stay untracked by recorded decision (module docs + docs/configuration.md): no enumerable record exists.
<!-- SECTION:NOTES:END -->
