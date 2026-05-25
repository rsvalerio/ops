---
id: TASK-1253
title: >-
  ERR-2: dep_count keyed by package_name drops counts for renamed or
  duplicate-named workspace crates
status: Done
assignee:
  - TASK-1267
created_date: '2026-05-08 13:01'
updated_date: '2026-05-09 14:55'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:80`

**What**: `RustUnitsProvider::provide` looks up `dep_counts.get(pn)` using the per-crate Cargo.toml `[package].name`. The `crate_dependencies` view is keyed by metadata `crate_name`, which differs when a member uses `package = "alt-name"` or two members share a name (the duplicate-name case is documented at `metadata/types.rs::package_index_by_name` TASK-1019).

**Why it matters**: For renamed or duplicate-named workspace members, `dep_count` silently mis-attributes or returns None with no warn, while sibling failure classes already route through tracing::debug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Key dep_counts by member manifest_path or metadata id instead of bare name
- [x] #2 tracing::debug breadcrumb when the same name maps to multiple packages
- [x] #3 Unit test with two members both named lib but different parent paths
<!-- AC:END -->
