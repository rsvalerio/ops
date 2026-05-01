---
id: TASK-0804
title: >-
  ERR-2: RustUnitsProvider dep_counts HashMap collapses package-name lookup
  misses into None silently
status: Triage
assignee: []
created_date: '2026-05-01 06:01'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:38-49`

**What**: dep_counts is keyed by package_name extracted from read_crate_metadata. If the member crate Cargo.toml is missing or unparseable, pkg_name.unwrap_or_default() becomes the empty string, and dep_counts.get(empty-string) always returns None — but the unit silently shows dep_count = None rather than surfacing the manifest issue. Sister DuckDB call sites use query_or_warn for query failures; this site has no equivalent.

**Why it matters**: Operators looking at dependencies em-dash on a unit card cannot distinguish DuckDB has no row from we silently fell back to empty package_name and the join can never match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When the resolved package_name is empty (manifest missing/unparseable), return None for dep_count and emit a tracing::debug
- [ ] #2 Alternatively, key dep_counts by member path so the join cannot collapse on missing names
- [ ] #3 Unit test asserting that a malformed member Cargo.toml produces a unit with dep_count None and one tracing::debug record (no panic, no silent zero)
<!-- AC:END -->
