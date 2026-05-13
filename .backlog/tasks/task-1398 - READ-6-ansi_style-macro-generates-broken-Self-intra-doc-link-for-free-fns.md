---
id: TASK-1398
title: 'READ-6: ansi_style! macro generates broken Self:: intra-doc link for free fns'
status: To Do
assignee:
  - TASK-1458
created_date: '2026-05-13 18:09'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/style.rs:50,52`

**What**: The generated rustdoc for the gated variant says ``Same as [`Self::$name`]`` — but the macro expands to free functions, not associated methods, so `Self::` has no meaning and the intra-doc link resolves to nothing.

**Why it matters**: Under `cargo doc -- -D rustdoc::broken-intra-doc-links` this fails the documentation lint baseline. Replace the link target with ``[`$name`]`` so it resolves against the module path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Macro-generated doc links resolve under broken-intra-doc-links lint
<!-- AC:END -->
