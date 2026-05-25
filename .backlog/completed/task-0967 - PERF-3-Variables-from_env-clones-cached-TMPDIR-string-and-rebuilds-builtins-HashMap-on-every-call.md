---
id: TASK-0967
title: >-
  PERF-3: Variables::from_env clones cached TMPDIR string and rebuilds builtins
  HashMap on every call
status: Done
assignee: []
created_date: '2026-05-04 21:47'
updated_date: '2026-05-04 22:57'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:75-83`

**What**: `Variables::from_env(ops_root)` allocates a fresh `HashMap<&'static str, String>` and clones the OnceLock-cached TMPDIR string on every call. `from_env` is invoked outside the parallel-runtime Arc-cloning boundary (CLI entry, hooks, RunBeforeCommit), reproducing the same payload N times.

**Why it matters**: Avoidable allocations on common entry paths. A LazyLock-built static, or returning a borrow from a cached String, amortizes to zero per construction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Variables::from_env reuses a cached TMPDIR Arc<str> (or equivalent borrowed form) rather than .clone()-ing the OnceLock-stored String
- [ ] #2 Public API of Variables::expand / try_expand is unchanged
- [ ] #3 Microbench-style regression test pins the no-fresh-allocation behavior
<!-- AC:END -->
