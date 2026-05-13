---
id: TASK-1415
title: >-
  API-1: CommandSpec lacks a unified accessor for category/help; callers match
  Exec/Composite manually
status: To Do
assignee:
  - TASK-1456
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:24`

**What**: `CommandSpec::help`, `category`, and `aliases` each `match self` over the Exec/Composite variants. The match arms duplicate identical extract-and-return shapes three times in `commands.rs:26-47` and similar matches recur across help.rs and other callers. Adding a variant requires touching every match site.

**Why it matters**: Low-impact maintainability — a future variant addition is a churn vector. Either add a macro to derive the helpers or introduce a `CommandMeta` trait the variants implement so dispatch is structural rather than three parallel match blocks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 reduce the help/category/aliases match-arm triplication via a shared accessor or macro
- [ ] #2 preserve the exact return shapes used by current callers (&str / Option<&str>)
- [ ] #3 regression covered by existing command-spec tests (no behavioural change)
<!-- AC:END -->
