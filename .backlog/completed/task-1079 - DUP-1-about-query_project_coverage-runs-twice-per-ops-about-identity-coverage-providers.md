---
id: TASK-1079
title: >-
  DUP-1: about query_project_coverage runs twice per 'ops about' (identity +
  coverage providers)
status: Done
assignee: []
created_date: '2026-05-07 21:20'
updated_date: '2026-05-08 06:19'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/metrics.rs:62` and `extensions-rust/about/src/coverage_provider.rs:44`

**What**: Identity metrics calls `query_project_coverage` once for `coverage_percent`, and `RustCoverageProvider` calls it again to drive the per-unit breakdown. Both providers run for one `ops about` invocation, so the same DuckDB scan + `query_or_warn` schema-drift handling fires twice with the same payload.

**Why it matters**: Each call is cheap individually but the duplicated handling means a schema-drift warn fires twice, doubling log volume. A future, more expensive query would scan twice. Lower than PERF-1 because the absolute cost is small, but the warn-doubling is operator-visible noise.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the project_coverage result on Context (similar to typed_manifest_cache)
- [ ] #2 identity::metrics consumes the cached value rather than re-querying
- [ ] #3 Test that the warn fires exactly once on a forced query failure during a single 'ops about'
<!-- AC:END -->
