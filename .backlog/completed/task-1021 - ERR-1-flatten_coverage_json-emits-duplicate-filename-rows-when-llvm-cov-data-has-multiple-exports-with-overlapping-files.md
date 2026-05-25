---
id: TASK-1021
title: >-
  ERR-1: flatten_coverage_json emits duplicate filename rows when llvm-cov data
  has multiple exports with overlapping files
status: Done
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-08 06:52'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:217-260`

**What**: TASK-0595 fixed the prior bug of dropping `data[1..]` by iterating every export. But the loop now appends one record per `(export, filename)` pair into `records` without any de-duplication or aggregation. If two exports list the same source file (the typical case for cargo llvm-cov when targets are merged across binaries/tests/examples), `coverage_files` ingests both rows under the same `filename` key and the downstream `coverage_summary` view double-counts `lines_count` / `lines_covered` for those files.

Today this appears to be latent because `cargo llvm-cov --workspace --tests --json` emits a single export, but the warn at line 198-203 explicitly anticipates multi-export inputs (`"flattening all entries"`) — the moment a future llvm-cov version or sibling caller passes multi-export JSON, the project totals double.

**Why it matters**: The percentage columns in `coverage_summary` come from SQL `SUM`s; duplicate rows inflate `lines_count` and `lines_covered` proportionally so the percentage stays plausible (still ~80%) but the absolute counts in `ops about coverage` are wrong. No diagnostic fires because the rows are valid and DuckDB has no PK on `coverage_files.filename`.

**Why it matters (sec)**: not a security issue; correctness/reporting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 flatten_coverage_json either deduplicates by filename across exports (last-wins or merge) or emits a structured error if duplicates are seen
- [x] #2 Unit test exercises a 2-export coverage JSON with one overlapping filename and pins the documented behaviour
<!-- AC:END -->
