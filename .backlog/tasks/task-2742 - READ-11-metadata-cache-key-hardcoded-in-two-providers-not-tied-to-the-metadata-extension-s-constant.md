---
id: TASK-2742
title: 'READ-11: "metadata" cache key hardcoded in two providers, not tied to the metadata extension''s constant'
status: Done
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 22:29'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/deps_provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-11:extensions-rust/about/src/units.rs:RustUnitsProvider::provide'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:64` (`RustUnitsProvider::provide`: `ctx.cached("metadata")`), `extensions-rust/about/src/deps_provider.rs:84` (`RustDuplicatesProvider::provide`: `ctx.cached("metadata")`)

**What**: Both providers read the cargo-metadata document via the string literal `"metadata"`. The producer's key is `DATA_PROVIDER_NAME` in `extensions-rust/metadata/src/lib.rs:41`, which is private. Every other cross-extension key in this crate goes through the producer's exported constant (`ops_cargo_toml::DATA_PROVIDER_NAME`, `ops_about::deps::PROJECT_DUPLICATES_PROVIDER`), with the latter's doc explicitly saying it is re-exported "so the renderer and this provider cannot drift".

**Why it matters**: A rename of the metadata provider compiles cleanly and silently turns the duplicates report empty and drops every unit's build targets, since a cache miss is the documented "not warmed" fallback.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The metadata provider key is a single named constant shared with the producer (exported from the metadata extension or a shared crate) and both call sites use it

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
