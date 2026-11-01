---
id: TASK-2650
title: 'ops config check is blind to field values and to behaviour changes no field marks'
status: To Do
assignee: []
created_date: '2026-11-01 00:41'
updated_date: '2026-11-01 10:31'
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
- [ ] #1 A value or behaviour newer than the target is reported by ops config check, or the decision not to track it is recorded
<!-- AC:END -->
