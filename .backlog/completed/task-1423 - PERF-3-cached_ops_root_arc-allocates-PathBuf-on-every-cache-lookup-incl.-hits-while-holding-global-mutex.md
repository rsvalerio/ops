---
id: TASK-1423
title: >-
  PERF-3: cached_ops_root_arc allocates PathBuf on every cache lookup (incl.
  hits) while holding global mutex
status: Done
assignee:
  - TASK-1455
created_date: '2026-05-13 18:22'
updated_date: '2026-05-13 22:59'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:120`

**What**: `guard.entry(ops_root.to_path_buf())` allocates a fresh `PathBuf` for every call, even when the key is already present. The allocation happens inside the global mutex critical section.

**Why it matters**: `from_env` is on the about-card / dry-run / hook path; the PathBuf clone defeats half of the memoisation intent (avoid String allocation, but still allocate PathBuf per call) and lengthens lock-hold time. Combined with CONC-1 / task-1418 (unbounded cache) this also amplifies churn.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use guard.get(ops_root) first; only fall through to entry(to_path_buf()) on a miss
- [x] #2 Microbench or test pinning that the hit path does not allocate a PathBuf
<!-- AC:END -->
