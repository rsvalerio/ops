---
id: TASK-2831
title: 'CL-3: per-crate LOC, file-count and coverage queries double-count files of nested workspace members'
status: To Do
assignee: []
created_date: '2026-11-18 18:50'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/query/loc.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:extensions/sqlite/src/sql/query/helpers.rs:query_per_crate_i64'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/query/helpers.rs:330-362` (`query_per_crate_i64`, used by `query_crate_loc` / `query_crate_file_count` in `sql/query/loc.rs`), `extensions/sqlite/src/sql/query/coverage.rs:49-113` (`query_crate_coverage`)

**What**: Each member is joined to every file whose path starts with `m.path || '/'`, and rows are grouped per member. When one member's directory contains another member (for example `crates/foo` and `crates/foo/macros`, or a `fuzz/` crate inside a package), every file under the inner member matches both prefixes and is counted in both crates' LOC, file count and coverage totals. `CrateCoverage`'s doc states "regions do not overlap, each source line is attributed to exactly one crate", which is false for nested members. The crate already knows this hazard: `query_rust_loc_files` documents that per-crate attribution must use the longest member-path prefix, and does it in Rust for exactly this reason. The SQL helpers do not apply it.

**Why it matters**: The per-crate numbers on the about/units pages are inflated for any workspace with nested members, and they no longer sum to the project totals rendered next to them. Nothing warns about it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each file is attributed to at most one member, the one with the longest matching member-path prefix, in query_crate_loc, query_crate_file_count and query_crate_coverage
- [ ] #2 A test with members 'a' and 'a/b' asserts a file under a/b/ counts only toward 'a/b' and that per-crate sums equal the project total
- [ ] #3 The CrateCoverage doc claim of non-overlapping attribution holds or is corrected
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
