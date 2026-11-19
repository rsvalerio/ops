---
id: TASK-2740
title: 'READ-13: manifest_cache module docs carry migration plans and reviewer rules instead of the contract'
status: To Do
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/workspace_root_cache.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/manifest_cache.rs:module-doc'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest_cache.rs:1-100` (module `//!` docs, sections "Concurrency contract" / "Reviewer rule" / staleness rule), `extensions-rust/about/src/manifest.rs:102-110` (`LoadedManifest::unexpanded_workspace_members_spec` doc: "which is precisely what the old `Deref` into `CargoToml` allowed"), `extensions-rust/about/src/workspace_root_cache.rs:32-38` (module docs repeating the migration note)

**What**: The module docs mix the enduring cache contract (key, value, cap, invalidation, member-set freeze) with process material: a future migration plan ("the cache MUST migrate to `DashMap<PathBuf, TypedManifestEntry>` plus a separate `parking_lot::Mutex<()>`"), a "Reviewer rule: do not add a daemon caller without first making the migration above", "would silently undo a downstream performance fix", "keep this comment in sync with extensions/about/src/manifest_cache.rs", and history of the removed `Deref` impl. The same migration note is repeated in `workspace_root_cache.rs`.

**Why it matters**: These read as authoritative API docs but describe a design journey and future work; they go stale on the next change (the sibling-cache sync instruction already depends on another crate's comment). The contract a caller needs is buried in ~100 lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module/item docs keep only the end-state contract (key, value, cap, invalidation, member-set freeze, single-threaded-host assumption)
- [ ] #2 Migration plans, reviewer rules and history of the removed Deref move to a backlog task, ADR or PR description
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
