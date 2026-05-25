---
id: TASK-1357
title: >-
  OWN-8: preprocess_args clones args[0] (OsString) immediately before consuming
  args via into_iter
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-12 21:28'
updated_date: '2026-05-17 09:14'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:241`

**What**: `preprocess_args` clones `args[0]` to satisfy the borrow checker, then moves the remainder of `args` through `into_iter`. The first element is an `OsString` (potentially a long argv[0] path on Unix).

**Why it matters**: Pure mechanical clone-to-appease-borrowck. The same shape is expressible without allocation by consuming the iterator: `let mut it = args.into_iter(); let first = it.next().unwrap(); /* skip program */ std::iter::once(first).chain(it)`. OWN-8 (unnecessary clone). Low-cost fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rewrite preprocess_args so args[0] is moved out via .next() instead of cloned
- [x] #2 args::tests pass unchanged; no new clippy::redundant_clone or clippy::needless_collect warnings
<!-- AC:END -->
