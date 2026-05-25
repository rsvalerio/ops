---
id: TASK-1350
title: >-
  PATTERN-1: classify_collision has a defensive Owner::Extension(_)
  self-collision branch whose policy-dependent return value is dead code
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:42'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/registration.rs:171-178`

**What**: The `Owner::Extension(_) if *prev == ext_name` branch returns `matches!(audit.policy, InsertPolicy::LastWriteWins)` — but the inline comment acknowledges in-extension duplicates have already been drained by `take_duplicate_inserts` in the caller before this loop iterates `local`. By construction, each id in `local` is unique per extension, so this branch is unreachable for the command and data-provider paths.

**Why it matters**: Defensive code that pretends to encode policy ("returns LWW true, FWW false") for a state the invariant excludes obscures the actual contract. A `debug_assert!(prev != ext_name, ...)` or `unreachable!("in-extension duplicates drained upstream")` makes the invariant explicit; the current shape lets a future refactor that removes the drain silently re-introduce double-counted collisions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Self-collision branch either replaced with debug_assert!/unreachable! reflecting the drained-upstream invariant, or covered by a regression test
- [ ] #2 cargo test --workspace passes
<!-- AC:END -->
