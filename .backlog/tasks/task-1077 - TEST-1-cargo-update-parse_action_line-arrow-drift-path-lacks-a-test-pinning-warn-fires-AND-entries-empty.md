---
id: TASK-1077
title: >-
  TEST-1: cargo-update parse_action_line arrow-drift path lacks a test pinning
  warn-fires AND entries-empty
status: Done
assignee: []
created_date: '2026-05-07 21:20'
updated_date: '2026-05-08 06:35'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:236-258`

**What**: `parse_action_line` for `Updating` requires `arrow == "->"`. On drift (e.g. `Updating serde v1.0.0 to v1.0.1`) it returns None and `starts_with_known_verb` re-fires the warn. No test asserts both invariants together (warn fires AND entries.is_empty()).

**Why it matters**: TEST-1 — the observability contract is unverified. A refactor could swallow the warn silently (e.g. by short-circuiting the verb match), and existing tests would not notice. Sister to TASK-1054 / TASK-1028 logic but for a different drift shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test asserts warn fires for an Updating line missing -> AND entries.is_empty() (capture via tracing-subscriber)
- [x] #2 Same coverage for Adding / Removing with extra trailing tokens
<!-- AC:END -->
