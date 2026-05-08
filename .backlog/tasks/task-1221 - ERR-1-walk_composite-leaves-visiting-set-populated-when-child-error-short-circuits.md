---
id: TASK-1221
title: >-
  ERR-1: walk_composite leaves visiting set populated when child error
  short-circuits
status: To Do
assignee:
  - TASK-1268
created_date: '2026-05-08 12:57'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:138-170`

**What**: `walk_composite` calls `visiting.insert(name)` and only removes on the success path. Early Err returns leave the entry, so the contract becomes "visiting is consistent only on Ok". Each top-level call constructs a fresh HashSet so today's leak is contained, but the invariant is fragile.

**Why it matters**: A future refactor that hoists `visiting` to `validate_commands`'s outer scope (an obvious optimisation) would silently produce false-positive cycle errors on re-validation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use a scope-guard / RAII removal pattern
- [ ] #2 OR document the on-error-leak invariant in the function rustdoc
- [ ] #3 Add a test asserting visiting is empty after an error inside the recursion
<!-- AC:END -->
