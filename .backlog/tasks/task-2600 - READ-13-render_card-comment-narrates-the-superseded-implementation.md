---
id: TASK-2600
title: 'READ-13: render_card comment narrates the superseded implementation'
status: To Do
assignee: []
created_date: '2026-10-10 20:48'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/cards.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/about/src/cards.rs:render_card'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:91`

**What**: The inline comment above the `title` binding in `render_card` narrates the change that produced the code: "Borrow `unit.name` directly when no version suffix is needed (PERF-3 / OWN-8): **the prior code cloned** the name into an owned String even when the `format!` call was unreachable." The rule-ID citation and the "prior code" narration are process artifacts from the fixing PR; what endures is only why borrowing is correct here.

**Why it matters**: READ-13: docs must describe the end state, not the journey. "The prior code cloned X" goes stale on the next change and is meaningless to a reader using the API. Three prior READ-13 waves (TASK-2261/2397/2432) cleaned this crate; this comment survived them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comment states why the borrowed form is correct without referencing a prior implementation or review rule IDs
<!-- AC:END -->
