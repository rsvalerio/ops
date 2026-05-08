---
id: TASK-1251
title: >-
  PERF-3: RustUnitsProvider rebuilds Vec<&str> and sort_unstable per provide()
  despite resolved_members already sorted+deduped
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 13:01'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:67`

**What**: `resolved_workspace_members` returns sorted+deduped Vec<String> (TASK-1042 dedup + TASK-0794 sort). `RustUnitsProvider::provide` then reallocates a Vec<&str> via `map(String::as_str).collect()` and `sort_unstable()` it again before iterating.

**Why it matters**: Wasted allocation + sort on a per-`ops about` hot path. Sister providers (coverage_provider, identity) consume `resolved_members()` directly without re-sorting because the contract already guarantees order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop sorted_members and iterate members.iter() directly
- [ ] #2 Encode the ordering invariant on LoadedManifest::resolved_members rustdoc
- [ ] #3 Regression test pinning fixed traversal order across two provide() calls
<!-- AC:END -->
