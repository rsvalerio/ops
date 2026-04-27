---
id: TASK-0371
title: 'PERF-1: prepare_per_crate clones member_paths twice for each call'
status: Done
assignee:
  - TASK-0421
created_date: '2026-04-26 09:37'
updated_date: '2026-04-27 16:08'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:178`

**What**: prepare_per_crate collects member_paths.iter().map(|p| p.to_string()).collect() into a fresh Vec<String> for binding; query_crate_coverage then paths.push(workspace_root.to_string()) after the helper returned. Each per-crate query allocates one String per member path even though all callers already hold &str slices.

**Why it matters**: Negligible for small workspaces but a per-query O(N) allocation pattern under repeated provider calls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bind paths via duckdb::params_from_iter(member_paths.iter().copied()) without intermediate Vec<String>
- [x] #2 Benchmark or microbench shows allocation reduction (or at minimum, no regression)
<!-- AC:END -->
