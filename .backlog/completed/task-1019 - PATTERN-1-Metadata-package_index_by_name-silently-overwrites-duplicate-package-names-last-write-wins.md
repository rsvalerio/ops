---
id: TASK-1019
title: >-
  PATTERN-1: Metadata::package_index_by_name silently overwrites duplicate
  package names, last-write-wins
status: Done
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-08 06:35'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:146-160` (and `package_index_by_id` 162-176, but ids are unique by construction)

**What**: `package_index_by_name` builds a `HashMap<String, usize>` keyed by package `name` only. In a real workspace, `cargo metadata`'s `packages[]` array can contain multiple entries with the same `name` but different versions/sources (transitive dependency resolved at multiple versions, or a workspace member shadowing a registry crate). The `.collect::<HashMap<_, _>>()` silently overwrites earlier entries — only the last package with a given name survives in the index. `package_by_name()` therefore returns an arbitrary one of N candidates with no signal.

The previous linear-scan code (TASK-0883 history) had the exact same defect (`Iterator::find` returns the first match), but converting it to a HashMap shipped without adding a uniqueness check or a tracing breadcrumb on collision.

**Why it matters**: Consumers calling `metadata.package_by_name("serde")` think they get *the* serde package; in a workspace with both `serde 1.0.x` and `serde 0.9.x` on the resolution graph (legacy member, optional feature path), the answer is non-deterministic across runs (HashMap insertion order is stable per process, but versions differ). About/units/coverage providers downstream silently report the wrong package's metadata.

<!-- scan confidence: candidate to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 package_index_by_name detects collisions and either fails closed (returns Err) or emits a tracing::warn and documents the non-determinism
- [x] #2 Add a unit test that builds a Metadata from a packages array containing two entries with the same name but distinct ids/versions and asserts the documented behaviour
<!-- AC:END -->
