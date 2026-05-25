---
id: TASK-1068
title: >-
  READ-5: TMPDIR_DISPLAY OnceLock makes Variables::from_env deaf to runtime
  TMPDIR changes
status: Done
assignee: []
created_date: '2026-05-07 21:18'
updated_date: '2026-05-07 23:29'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:78-89`

**What**: `TMPDIR_DISPLAY` is cached at first call via `OnceLock`; subsequent `set_var(\"TMPDIR\", ...)` is invisible to `Variables::from_env`. Tests that swap `TMPDIR` post-init silently observe stale data.

**Why it matters**: Same flake class as TASK-1037 (cached `Arc<str>`). The cache is a deliberate optimisation but the contract is undocumented, so tests that rely on swapping `TMPDIR` get inconsistent behaviour depending on call order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document that TMPDIR is read once per process for Variables::from_env
- [ ] #2 Add a serial test asserting the documented contract (post-init set_var is not observed)
- [ ] #3 Consider exposing Variables::from_env_uncached for tests that need fresh reads
<!-- AC:END -->
