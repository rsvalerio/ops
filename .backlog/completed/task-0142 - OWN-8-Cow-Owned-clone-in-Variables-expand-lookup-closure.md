---
id: TASK-0142
title: 'OWN-8: Cow::Owned clone in Variables::expand lookup closure'
status: Done
assignee: []
created_date: '2026-04-22 21:17'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - own
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:38`

**What**: The shellexpand lookup closure returns `Cow::Owned(val.clone())` for every successful lookup, even though the closure borrows from a HashMap that outlives the expansion call and could return Cow::Borrowed.

**Why it matters**: One heap allocation per expanded variable per command spec; not a hot path but a clear ownership-design improvement.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change the closure to return Cow::Borrowed(val.as_str()) when the value comes from the HashMap
- [ ] #2 Verify the lifetime compiles against shellexpand::full_with_context signature
<!-- AC:END -->
