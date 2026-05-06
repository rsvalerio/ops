---
id: TASK-0984
title: >-
  ERR-1: test-coverage flatten_coverage_json silently coerces missing filename
  to empty string, polluting coverage_files with an unkeyed row
status: To Do
assignee:
  - TASK-1013
created_date: '2026-05-04 21:58'
updated_date: '2026-05-06 06:48'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:218`

**What**: Inside `flatten_coverage_json`, every other field uses `read_field` / `read_i64_field` / `read_f64_field`, which warn at `tracing::warn` when the field is present but the wrong shape (schema drift surfaces). `filename` is the exception:
```rust
let filename = file.get("filename").and_then(|f| f.as_str()).unwrap_or("");
```
A missing or non-string `filename` collapses to an empty string with no breadcrumb. The flattened record still gets pushed into `coverage_files`, which then participates in the `coverage_summary` aggregation and per-file UnitCoverage joins (`extensions-rust/about/src/coverage_provider.rs:84`) keyed by member path — the empty-key row will not match any member but still inflates the project-total `lines_count` / `lines_covered` totals.

**Why it matters**: llvm-cov schema drift on `filename` (or a future change that emits null for ignored files) would silently double-count or otherwise pollute the project-coverage signal feeding `ops about`'s primary health badge, with zero log evidence. Sister fields routinely warn on drift via the existing `read_field` helper — `filename` should follow the same convention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Missing or non-string filename either skips the file with a tracing::warn breadcrumb or surfaces the per-file shape error via the same read_field-style helper
- [ ] #2 Regression test feeds llvm-cov-shaped JSON with one file lacking filename and asserts the warn fires AND the aggregate project total excludes that record
<!-- AC:END -->
