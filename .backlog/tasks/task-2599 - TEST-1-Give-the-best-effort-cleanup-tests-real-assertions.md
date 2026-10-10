---
id: TASK-2599
title: 'TEST-1: Give the best-effort cleanup tests real assertions'
status: To Do
assignee: []
created_date: '2026-10-10 15:48'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/sql/ingest/sidecar.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
  - extensions/sqlite/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-1:extensions/sqlite/src/ingestor.rs:cleanup_is_best_effort_when_json_missing'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**Files/candidates**:
- `extensions/sqlite/src/ingestor.rs:679` `cleanup_is_best_effort_when_json_missing` — body stages only the sidecar, calls `config.cleanup_artifacts(&dir)`, then ends with the comment "// Sidecar removal should still complete." The named outcome is never asserted; assert the sidecar entry is gone (`!dir.entry_path(...).exists()`) after cleanup.
- `extensions/sqlite/src/sql/ingest/sidecar.rs:291` `workspace_sidecar_remove_is_best_effort` — a single `remove_workspace_sidecar(&dir, "missing_name")` call; the only property verified is "did not panic". Its sister test (`..._logs_but_does_not_panic_on_failure`) shows the assertion style available (check directory state after the call).
- `extensions/sqlite/src/sql/ingest/orchestrator.rs:599` `reentry_guard_allows_distinct_tables_on_same_thread` — creates two guards and drops them; the distinct-tables-allowed property is implicit in not panicking. Strengthen by e.g. dropping `_a` and re-acquiring the same table name to also pin release-then-reacquire.
- `extensions/sqlite/src/lib.rs:212` `sqlite_lock_returns_guard` — lock + drop with only setup `expect`s; weakest candidate (a duplicate of the same-named test in connection.rs's own module, which at least runs under richer company).

**Why it matters**: TEST-1: a test with no assertion only proves "did not panic", so a regression that quietly stops performing the cleanup/removal the test name promises still passes. Each candidate states its expected outcome in a comment or name but never checks it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed test contains at least one explicit assertion of the outcome its name or trailing comment states (sidecar removed after best-effort cleanup; directory state after removal of a missing sidecar; release-then-reacquire for the reentry guard)
<!-- AC:END -->
