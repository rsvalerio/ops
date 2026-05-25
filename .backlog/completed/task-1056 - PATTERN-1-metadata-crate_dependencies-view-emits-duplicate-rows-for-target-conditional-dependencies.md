---
id: TASK-1056
title: >-
  PATTERN-1: metadata crate_dependencies view emits duplicate rows for
  target-conditional dependencies
status: Done
assignee: []
created_date: '2026-05-07 21:03'
updated_date: '2026-05-08 06:52'
labels:
  - code-review
  - extensions-rust
  - metadata
  - PATTERN-1
  - ERR-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/metadata/src/views.rs:17-30 builds the crate_dependencies view as: SELECT crate_name, dep.name AS dependency_name, dep.req AS version_req, COALESCE(dep.kind, 'normal') AS dependency_kind, COALESCE(dep.optional, false) AS is_optional FROM member_deps. cargo metadata emits one row per (name, kind, target) triple in package.dependencies — a target-conditional dep declared twice ([target.'cfg(windows)'.dependencies] + [target.'cfg(unix)'.dependencies]) for the same crate appears as two distinct dependency entries. The view drops dep.target, so both rows surface as identical (crate_name, dependency_name, version_req, dependency_kind, is_optional) tuples and inflate dependency counts in downstream consumers (about/deps_provider, identity dependency_count, deps reporting). The same shape happens when one declaration is in [dependencies] and another in [target.<...>.dependencies] with matching kind/req.

Sister to TASK-0982 (path-deps drop). This is the inverse: duplicate retention.

Fix options: include dep.target in the SELECT and downstream consumers (preferred — preserves platform-conditional shape); collapse with DISTINCT once justification exists; or aggregate target into an array column.

Add a regression test against a fixture metadata.json with a target-conditional dep declared twice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decide whether crate_dependencies should preserve target (preferred) or DISTINCT
- [x] #2 Implement the chosen fix and update downstream queries (query_dependency_count, query_crate_dep_counts, query_crate_deps)
- [x] #3 Regression test feeds a fixture with the same dep declared under two cfg-targets and pins the row count
<!-- AC:END -->
