---
id: TASK-2875
title: 'CONC: about-rust manifest and workspace-root caches need sharding and member-set freshness before any parallel or long-lived host'
status: To Do
assignee: []
created_date: '2026-11-18 22:30'
updated_date: '2026-11-19 07:57'
labels:
  - code-review-rust
  - concurrency
dependencies: []
parent_task_id: 'TASK-2883'
modified_files:
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/workspace_root_cache.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest_cache.rs`, `extensions-rust/about/src/workspace_root_cache.rs`

**What**: Both caches are a single process-global `Mutex<HashMap>` and assume a single-threaded host (the CLI). The typed-manifest cache's freshness key stats only `<root>/Cargo.toml`, so `resolved_members` and `canonical_member_manifests` are frozen for an entry's lifetime. This planning material previously lived in the module docs (removed as process material under READ-13):
- Before a daemon / language-server / parallel CI host calls these providers concurrently against many roots, migrate both caches to a sharded map (e.g. `DashMap<PathBuf, TypedManifestEntry>` plus a small mutex only for the LRU cap-evict scan), matching the per-key `OnceLock` shape the sibling cache in `extensions/about/src/manifest_cache.rs` already uses.
- Such a host must also either extend the freshness key to cover the resolved member set or drive `ctx.refresh` on workspace-modification events, or it serves a stale member list with no warn.
- Do not add such a caller without first making these changes.

**Why it matters**: Without the migration a parallel host serialises every provider on one mutex; without member-set freshness it diverges from `cargo metadata` silently.

**Origin**: discovered during TASK-2849 while fixing TASK-2740 (migration notes moved out of module docs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Before the first parallel or long-lived host caller lands, both caches are sharded and the member-set freshness gap is closed or driven by ctx.refresh
<!-- AC:END -->
