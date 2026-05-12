---
id: TASK-1356
title: >-
  PERF-3: classify_and_warn_collision allocates key.to_string() on every Entry
  lookup including the common hit path
status: Done
assignee: []
created_date: '2026-05-12 21:28'
updated_date: '2026-05-12 21:30'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/registration.rs:130`

**What**: `match owners.entry(key.to_string())` allocates a fresh `String` for every probe, but the steady-state common case is `Entry::Occupied` (no new owner). On the hit path the allocation is immediately dropped after the match.

**Why it matters**: The collision check runs once per (extension × command-or-data-provider) pair on every CLI invocation. Allocation per hit is wasteful and shows up in `cargo flamegraph` traces of registry hot frames. Probe with `.get`/`.contains_key` first and only allocate on the Vacant insertion arm; or use `raw_entry_mut`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor classify_and_warn_collision to avoid allocating key.to_string() on the Occupied path (probe first, allocate only on Vacant)
- [ ] #2 Existing registration tests pass byte-for-byte; collision warning behavior unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Duplicate of TASK-1349 (classify_and_warn_collision was renamed from classify_collision; same finding — owners.entry(key.to_string()) allocating on the Occupied hot path).
<!-- SECTION:NOTES:END -->
