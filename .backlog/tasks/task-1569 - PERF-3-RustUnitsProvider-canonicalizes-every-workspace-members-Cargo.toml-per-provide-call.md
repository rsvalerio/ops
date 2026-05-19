---
id: TASK-1569
title: >-
  PERF-3: RustUnitsProvider canonicalizes every workspace member's Cargo.toml
  per provide() call
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-19 16:35'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:110-111`

**What**: Inside `RustUnitsProvider::provide`, for each resolved workspace member the provider calls `std::fs::canonicalize(&crate_toml)` to build a dep-count lookup key. With N workspace members that is N filesystem syscalls (each walking symlinks for every ancestor) on every `ops about` invocation. Sibling caches (`typed_manifest_cache`, `cached_query_project_coverage`) explicitly amortise their per-provider work across the about pipeline; the canonicalize pass is the only un-amortised filesystem hit left on the units hot path.

**Why it matters**: For a workspace with dozens of crates this can dwarf the actual work of the provider, especially on networked FS / Windows where canonicalize is slow. The result is keyed by member path and is invariant for the duration of an `ops about` run, so a small `HashMap<&str, PathBuf>` (or a sibling-of-`LoadedManifest` resolved-canonical map populated once on the cache-miss path) would eliminate the per-call N-syscall fan-out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 RustUnitsProvider::provide does not call std::fs::canonicalize once per workspace member on every provide() invocation
- [ ] #2 Canonicalized manifest paths for the lookup key are computed at most once per workspace per ops about run (e.g. cached on LoadedManifest or in a sibling per-process cache)
- [ ] #3 rust_units_provider_handles_duplicate_named_members and provide_drops_absolute_and_traversal_members still pass
<!-- AC:END -->
