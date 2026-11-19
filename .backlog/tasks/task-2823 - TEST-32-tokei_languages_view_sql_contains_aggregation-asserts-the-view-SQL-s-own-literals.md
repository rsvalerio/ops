---
id: TASK-2823
title: 'TEST-32: tokei_languages_view_sql_contains_aggregation asserts the view SQL''s own literals'
status: To Do
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2855'
modified_files:
  - extensions/tokei/src/views.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions/tokei/src/views.rs:tokei_languages_view_sql_contains_aggregation'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/views.rs:74` (`tests::tokei_languages_view_sql_contains_aggregation`)

**What**: The test checks that `tokei_languages_view_sql()` contains `GROUP BY language`, `SUM(code)`, `COUNT(*)` and `ORDER BY code DESC`. These are verbatim fragments of the `&'static str` body passed in the function a few lines above. It passes by construction and fails only when someone edits the literal, at which point the test is edited in step. The behaviour it is meant to protect, per-language aggregation, is already verified against real rows by `tests.rs::tokei_languages_view_aggregates_correctly`.

**Why it matters**: A tautological test adds diff noise and false confidence without catching any regression the behavioural test misses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tokei_languages_view_sql_contains_aggregation is removed, or replaced by an assertion on behaviour (aggregated rows) not on the SQL text
- [ ] #2 cargo test -p ops-tokei passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.32 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
