---
id: TASK-1349
title: >-
  PERF-3: classify_collision allocates a String per call via
  owners.entry(key.to_string()) even on the common Occupied path
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:42'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/registration.rs:~130`

**What**: The collision-classification path constructs `owners.entry(key.to_string())` (or equivalent) which allocates a fresh `String` for every command/data-provider id, even when the entry is already `Occupied` (the common case — most extensions reuse owners snapshotted from `snapshot_initial_owners` at line 195). The collision warning is the only branch that needs the owned key for insertion.

**Why it matters**: Extension registration runs once at startup but loops `N extensions × M commands`. Using `raw_entry_mut` or doing a `get` + conditional `insert` avoids the per-key allocation on the Occupied path, which is the overwhelming majority. Mostly a cleanliness win, but it also makes the allocation cost proportional to actual collisions instead of every id.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Classification avoids String allocation on the Occupied (no-insert) path
- [ ] #2 Collision warnings still emit the correct key text; cargo test --workspace passes
<!-- AC:END -->
