---
id: TASK-0464
title: >-
  DUP-1: about subpages duplicate warm-providers + load-or-default scaffold
  across four files
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 05:46'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/{units.rs,coverage.rs,deps.rs,code.rs}` run_about_*_with

**What**: run_about_units_with, run_about_coverage_with, run_about_deps_with, and run_about_code build the same Context, iterate a `["duckdb", ...]` warm-up list with identical match-arms (Ok | Err(NotFound) silent, other -> tracing::debug), then call ctx.get_or_provide(<provider>, registry) with the same triadic match (Ok deserialize / NotFound default / Err return).

**Why it matters**: Each new subpage copies the boilerplate and its bug surface — units warms ["duckdb","tokei"] while coverage warms ["duckdb","coverage","cargo_toml"]; drift between lists is invisible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A new helper in about::providers consolidates warm-up + load-with-default; all four subpages call it
- [ ] #2 Behaviour is unchanged — existing subpage tests still pass without modification
<!-- AC:END -->
