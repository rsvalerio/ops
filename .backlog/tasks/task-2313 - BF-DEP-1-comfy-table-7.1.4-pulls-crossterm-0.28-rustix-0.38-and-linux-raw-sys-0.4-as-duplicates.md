---
id: TASK-2313
title: 'BF-DEP-1: comfy-table 7.1.4 pulls crossterm 0.28, rustix 0.38 and linux-raw-sys 0.4 as duplicates'
status: Triage
assignee: []
created_date: '2026-09-27 15:20'
labels:
  - rust-make-build-fast
  - build-fast
  - safe
  - dep
dependencies: []
modified_files:
  - Cargo.lock
priority: low
ordinal: 1000
dedup_key: 'BF-DEP-1:comfy-table'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Check**: `BF-DEP-1` — duplicate crate versions (**safe**)

**Where**: `Cargo.lock` (`comfy-table = "7"` in the root `Cargo.toml`, used by `crates/core` and `extensions-terraform/plan`)

**Evidence**: `ops about dependencies --duplicates --json` at d3a50ef7. comfy-table 7.1.4 is the only puller of the older version in three pairs, each with `updateRemovesDuplicate: true`:
- crossterm 0.28.1 (0.29.0 also in the graph)
- rustix 0.38.44 (1.1.4)
- linux-raw-sys 0.4.15 (0.12.1)

It also pulls windows-sys 0.59.0, which stays anyway through other pullers and is Windows-only here.

**Measured** 2026-09-27 · 16 cores, jobs=2 (~/.cargo/config.toml) · load 0.37 · sccache on · target on ext4 · cargo 1.98.0
- crates removed by the fix: 3 (compile time unmeasured: the survey ran in default mode, and no cold build was taken)

**Recommendation**: `cargo update -p comfy-table` to 7.2.x (satisfies `"7"`; 7.2 uses crossterm 0.29). It is a lockfile-only change. Check that table rendering (`ops about`, terraform plan output) is unchanged.

**Apply**: manual (dependency update, reviewed like code)

**Commit**: d3a50ef7
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `ops about dependencies --duplicates` no longer lists crossterm, rustix or linux-raw-sys with comfy-table as a puller
- [ ] #2 Behaviour is unchanged: the gates that passed before still pass
<!-- AC:END -->
