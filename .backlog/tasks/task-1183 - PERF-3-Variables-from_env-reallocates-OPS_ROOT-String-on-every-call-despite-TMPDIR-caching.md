---
id: TASK-1183
title: >-
  PERF-3: Variables::from_env reallocates OPS_ROOT String on every call despite
  TMPDIR caching
status: To Do
assignee:
  - TASK-1262
created_date: '2026-05-08 08:10'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:101`

**What**: `from_env` paid a `OnceLock<Arc<str>>` cache for TMPDIR (PERF-3 / TASK-0967) but still calls `ops_root.display().to_string()` then `Arc::<str>::from(_)` on every invocation. Each Variables build allocates a fresh String + Arc inner for the same project root.

**Why it matters**: Hooks (run-before-commit/run-before-push), about-card refreshes, and dry-run all call from_env; the asymmetry with the documented TMPDIR optimization is invisible to readers and defeats the "amortized to cache lookup" claim in the test from_env_amortises_tmpdir.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 from_env reuses an Arc<str> for the same ops_root path within a process (e.g. via a small LRU-of-1, OnceLock-per-root, or by taking Arc<Path>/&Arc<str> from the caller).
- [ ] #2 A test asserts Arc::ptr_eq on OPS_ROOT across two from_env(&same_root) calls, mirroring from_env_reuses_cached_tmpdir_arc.
<!-- AC:END -->
