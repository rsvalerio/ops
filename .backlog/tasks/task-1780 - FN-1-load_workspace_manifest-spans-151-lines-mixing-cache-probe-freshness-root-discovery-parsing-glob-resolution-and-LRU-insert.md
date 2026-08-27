---
id: TASK-1780
title: >-
  FN-1: load_workspace_manifest spans 151 lines mixing cache probe, freshness,
  root discovery, parsing, glob resolution and LRU insert
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - structure
dependencies: []
modified_files:
  - extensions-rust/about/src/query.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:346-497` (`load_workspace_manifest`)

**What**: 151 source lines, roughly 75 of them executable, at five distinct abstraction levels inside one body:

1. `:355` freshness stat of the on-disk `Cargo.toml`;
2. `:357-401` cache probe — three separate `lock_typed_manifest_cache` acquisitions across two branches, an mtime+len freshness comparison, an LRU tick refresh, an `Arc` clone, a manual `drop(guard)` and an early return;
3. `:423-445` two-way manifest acquisition — a `ctx.cached` JSON fast path that deserialises from a borrowed `Value`, versus a cold path that runs strict workspace-root discovery and builds a `CargoTomlProvider`;
4. `:452-461` glob resolution and `LoadedManifest` construction;
5. `:462-495` insert path — cap check, LRU eviction, tick mint, victim-queue push, map insert.

FN-1 asks for ≤50 lines at a single abstraction level. Each of the five blocks above is independently nameable (`probe_cache`, `load_manifest`, `insert_into_cache`) and each already carries a multi-paragraph comment explaining what it does — a reliable signal that the block wants to be a function with a doc comment instead.

The lock discipline is the concrete cost: the guard is acquired and released three times in three separate scopes (`:358`, `:361`, `:470`), one of them with an explicit `drop(guard)` at `:397` to make an early return work. Verifying "the lock is never held across IO or parsing" — the CONC-7 / TASK-1163 contract asserted at `:248-272` — currently requires reading the whole function; with the probe and the insert extracted, each scope would be checkable at a glance.

**Why it matters**: this function is the entry point every about provider goes through, it holds the crate's most delicate invariants (cache freshness, lock scope, glob-spec preservation), and it is the file's highest-churn body — `git log` shows it rewritten by TASK-0558, 0795, 0843, 0844, 0962, 1023, 1076, 1195, 1198, 1240 and 1572. CL-5 puts high-churn code firmly in the "minimise cognitive load" column.

Note: fixing the CL-3 workspace-root finding filed against the same function will add root-threading to this body; extracting the helpers first makes that change reviewable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_workspace_manifest is under 50 lines and reads as orchestration: probe the cache, load the manifest, insert into the cache
- [ ] #2 The cache probe (including freshness comparison and LRU tick refresh) and the cache insert (including cap check and eviction) are each a named function on TypedManifestCache or a free helper, so each lock scope is verifiable in isolation
- [ ] #3 No behaviour change: the existing query.rs cache tests (same-Arc reuse, refresh invalidation, mtime change, size change, bounded size, LRU-not-hot-key, cross-thread sharing, poison recovery) pass unmodified
<!-- AC:END -->
