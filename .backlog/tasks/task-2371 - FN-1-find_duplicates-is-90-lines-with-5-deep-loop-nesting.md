---
id: TASK-2371
title: 'FN-1: find_duplicates is ~90 lines with 5-deep loop nesting'
status: To Do
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - FN
dependencies: []
parent_task_id: 'TASK-2418'
modified_files:
  - extensions-rust/about/src/deps_provider.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/about/src/deps_provider.rs:find_duplicates'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/deps_provider.rs:415`

**What**: `find_duplicates` (lines 415-~503, ~90 lines) builds the package index and edges, computes reachability, groups versions by name, builds per-direct-dependency closures, then runs a `for name` > `for version` > `for puller` > `entries` loop nest (4-5 levels) that mixes graph analysis, dry-run caching and report assembly.

**Why it matters**: Exceeds FN-1 (50 lines) and FN-2 (nesting <= 4); the abstraction levels are mixed, so the dry-run memoization and the pulled_by construction cannot be unit-tested apart from the whole graph walk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 find_duplicates is <= 50 lines, delegating to named helpers (e.g. versions_by_name, direct_closures, pullers_of)
- [ ] #2 No loop nest deeper than 4 levels
- [ ] #3 Existing deps_provider tests pass unchanged
<!-- AC:END -->
