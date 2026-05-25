---
id: TASK-0969
title: >-
  PERF-3: resolved_workspace_members rebuilds member list on every call (cache
  amortization gap)
status: Done
assignee: []
created_date: '2026-05-04 21:48'
updated_date: '2026-05-04 23:00'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:222-316`

**What**: `resolved_workspace_members` is called via `load_workspace_manifest` (which caches the manifest in an Arc), but the resolved members list itself is not cached — every call rebuilds the HashSet, walks all glob `read_dir`, and re-sorts. The cache stores only the CargoToml; subsequent identity/units/coverage providers each recompute the same member list.

**Why it matters**: Repeated stat + read_dir + sort work for what `load_workspace_manifest` was supposed to amortize. Compounds in CI where multiple providers run sequentially.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the resolved-members list alongside the Arc<CargoToml> (e.g. Arc<(CargoToml, Vec<String>)>), invalidated by the same mtime check
- [ ] #2 Or memoize via a OnceCell on the cached entry
- [ ] #3 Trace or bench confirms the second call does not re-walk the filesystem
<!-- AC:END -->
