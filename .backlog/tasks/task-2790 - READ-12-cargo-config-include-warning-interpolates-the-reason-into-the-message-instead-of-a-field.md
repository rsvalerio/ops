---
id: TASK-2790
title: 'READ-12: cargo config include warning interpolates the reason into the message instead of a field'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'READ-12:extensions/about/src/machine.rs:include_paths'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:256-258` (`include_paths`, the `warn` closure)

**What**: `tracing::warn!(path = %layer.path.display(), "about/machine: ignoring cargo config include: {what}")` bakes the reason (`not an array`, `path does not end in .toml`, ...) into the message text, so each reason produces a distinct message string.

**Why it matters**: Subscribers and collectors cannot group or filter these warnings on the reason; READ-12 asks for a fixed message template with the variable part as a named field (`reason = what`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The include warning uses a constant message with the reason recorded as a named structured field
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
