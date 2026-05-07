---
id: TASK-1108
title: >-
  TEST-1: stray #[test] attribute and misaligned doc comment in
  extensions/git/src/config.rs around line 411-423
status: Done
assignee: []
created_date: '2026-05-07 21:35'
updated_date: '2026-05-07 23:15'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:417-423`

**What**: Lines 411-417 carry a doc comment intended for `origin_section_present_but_no_url_returns_none` (the function declared at line 450 — already in triage as TASK-1016 for missing `#[test]`). Above the doc comment a stray `#[test]` is left at line 417, and another `#[test]` follows at line 423 directly above `parse_section_header_unknown_escape_returns_typed_error`. The double-attribute block compiles, but the doc-comment-to-function pairing is misleading and obscures the TASK-1016 fix when it lands.

**Why it matters**: Distinct from TASK-1016 (which only files the missing attribute on the *other* function). Test/source readability.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove the orphaned #[test] at line 417 and reattach the doc comment to the function it actually documents
- [ ] #2 After the TASK-1016 fix, the file compiles cleanly with each #[test] attached to exactly one function and each doc comment immediately preceding its target
- [ ] #3 cargo doc --no-deps for ops_git regenerates with the doc comment under the right symbol
<!-- AC:END -->
