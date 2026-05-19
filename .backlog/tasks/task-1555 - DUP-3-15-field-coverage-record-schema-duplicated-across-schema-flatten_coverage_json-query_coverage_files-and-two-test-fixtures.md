---
id: TASK-1555
title: >-
  DUP-3: 15-field coverage record schema duplicated across schema(),
  flatten_coverage_json, query_coverage_files, and two test fixtures
status: To Do
assignee:
  - TASK-1577
created_date: '2026-05-19 15:34'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs` (multiple), `extensions-rust/test-coverage/src/ingestor.rs:60-78`, `extensions-rust/test-coverage/src/tests.rs:257-263`

**What**: The 15-field coverage row schema (`filename`, `lines_count`, `lines_covered`, `lines_percent`, `functions_*`, `regions_*` + `regions_notcovered`, `branches_*` + `branches_notcovered`) is restated in five places:

1. `CoverageProvider::schema` field list — `lib.rs:62-84`
2. `flatten_coverage_json` `serde_json::json!` record — `lib.rs:279-295`
3. `query_coverage_files` SELECT projection + `serde_json::json!` row builder — `lib.rs:354-378`
4. `coverage_load_with_sample_data` test fixture — `ingestor.rs:60-78`
5. `coverage_summary_view_handles_zero_counts` test fixture — `tests.rs:257-263`

Each is hand-rolled. Adding (or renaming) a single metric — e.g. `mcdc_*` if llvm-cov adds it — requires editing five call sites, with no compiler error if one is missed. `query_coverage_files` already drifted once before (`filename` was a `String` mismatch fix in TASK-0808).

**Why it matters**: Classic DUP-3 — \"identical schema literal repeated 5x.\" The natural fix is a `#[derive(Serialize, Deserialize)]` `CoverageRow` struct that lib.rs's flatten builds (via `serde_json::to_value`), `query_coverage_files` deserialises into, and the test fixtures construct with named-field syntax. Bonus: `schema()` could derive from the struct via a `Field`-collecting helper, eliminating duplication 1.

<!-- scan confidence: manually verified all five sites carry the full 15-field set -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single CoverageRow struct (or equivalent typed representation) is the source of truth for the 15-field row
- [ ] #2 flatten_coverage_json builds rows via the typed representation rather than an ad-hoc json! literal
- [ ] #3 query_coverage_files projects through the typed representation rather than restating the 15 column reads
- [ ] #4 Existing tests continue to pass without rewriting their assertions
<!-- AC:END -->
