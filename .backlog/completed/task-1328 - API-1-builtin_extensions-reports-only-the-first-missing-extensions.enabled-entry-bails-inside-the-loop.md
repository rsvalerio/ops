---
id: TASK-1328
title: >-
  API-1: builtin_extensions reports only the first missing extensions.enabled
  entry; bails inside the loop
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:21'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/discovery.rs:118-132`

**What**: The validation loop iterates `enabled` and calls `anyhow::bail!` on the *first* name absent from `available`. If the user has three typos in `extensions.enabled`, they fix one, re-run, see the next, fix, re-run, see the third. Each "rebuild + run + edit" cycle is human-paced and avoidable: the loop has all the information needed to surface every missing entry in a single error.

**Why it matters**: Trivial UX papercut, but it compounds — `extensions.enabled` is the documented surface for narrowing extension load order, and operators copy entries verbatim from blog posts / examples. Aggregating all misses (and emitting `available` once) turns N reload cycles into one.

Fix shape: collect `missing: Vec<&str>` across the loop and bail after the loop if non-empty, with the list embedded in the error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 builtin_extensions aggregates all missing extensions.enabled entries into one error
- [ ] #2 the available-names list is rendered once, not once per missing name
- [ ] #3 test asserts that an enabled list with multiple typos names every missing entry
<!-- AC:END -->
