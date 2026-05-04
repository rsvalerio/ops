---
id: TASK-0968
title: >-
  PERF-3: prepare_per_crate allocates intermediate Vec<&str> on every per-crate
  query
status: Triage
assignee: []
created_date: '2026-05-04 21:48'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:191`

**What**: `prepare_per_crate` builds the placeholder string via `member_paths.iter().map(|_| "(?)").collect::<Vec<_>>().join(", ")`. The Vec<&'static str> is unused after the join. Hit per query for query_crate_loc/file_count/coverage — multiple times per about-units render.

**Why it matters**: Trivial allocation eliminated by `String::with_capacity(n*4)` + `for _ in 0..n { push_str("(?), ") }`. Hot path for any about-units render against a workspace with N crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Placeholder build uses String::with_capacity + push without an intermediate Vec
- [ ] #2 Existing query_per_crate_i64 tests pass
<!-- AC:END -->
