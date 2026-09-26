---
id: TASK-2288
title: 'Add ops about dependencies --duplicates: real duplicate versions and whether an update removes them'
status: Done
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 19:08'
labels:
  - feature
  - about
  - deps
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2292'
modified_files:
  - extensions/about/src/deps.rs
  - extensions-rust/about/src/deps_provider.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops about dependencies --duplicates [--json]` lists crates that are locked at two or more **distinct** versions, excluding dev-only ones by default. For each, it names the workspace dependency that pulls the older version, and says whether a semver-compatible update of that dependency removes the duplicate. It checks this without writing `Cargo.lock`.

**Why**: `cargo tree -d` also lists crates that have one version reached through several paths. In the ops and dbsec evals it listed 24 and 33 names for 17 and 13 real duplicates. The skill then runs `cargo update --dry-run -p` per dependency to separate fixable duplicates from upstream-bound ones. Filing the upstream-bound ones produced eight tasks with no possible fix. Example of a fixable one found here: comfy-table 7.1.4 → 7.2.2 removes the crossterm 0.28, rustix 0.38 and linux-raw-sys 0.4 duplicates.

Check first whether `ops deps` (cargo-deny `bans`, multiple-versions) already covers part of this.

**Used by**: rust-make-build-fast (DEP-1).
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Only crates with two or more distinct versions are listed
- [x] #2 Each duplicate names its pulling workspace dependency and whether a semver-compatible update removes it
- [x] #3 Cargo.lock is never written

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped ops about dependencies --duplicates [--json] (kind about-dependency-duplicates): non-dev reachable crates with >=2 distinct versions from cargo metadata resolve; older versions name pulling direct deps; cargo update --dry-run -p name@version per puller (Cargo.lock verified unchanged). ops deps (cargo-deny bans) covers multiple-versions detection only, not pullers/fixability. Verified: comfy-table 7.1.4 update removes crossterm 0.28 / rustix 0.38 / linux-raw-sys 0.4. Dev-duplicate toggle filed as TASK-2299.
<!-- SECTION:NOTES:END -->
