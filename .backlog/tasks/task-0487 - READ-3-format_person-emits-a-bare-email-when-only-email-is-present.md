---
id: TASK-0487
title: 'READ-3: format_person emits a bare email when only email is present'
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:08'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:124-129`

**What**: For PersonField::Object { name: None, email: Some(e) }, the function returns Some(e) — the email alone, no surrounding angle brackets. The Text and (Some, Some) branches produce `Name <email>` style strings, so this branch silently emits a different shape.

**Why it matters**: Downstream renderers display these strings as authors; an unwrapped email is indistinguishable from a missing name. Formatting as <email> (or skipping) keeps the shape consistent and machine-recognisable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Return Some(format!("<{e}>")) for the email-only branch (or drop it intentionally)
- [ ] #2 Add a unit test for { "email": "a@example.com" } author input
- [ ] #3 Existing tests stay green
<!-- AC:END -->
