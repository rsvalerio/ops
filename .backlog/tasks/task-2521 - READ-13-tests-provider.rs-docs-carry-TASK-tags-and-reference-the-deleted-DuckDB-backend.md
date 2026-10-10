---
id: TASK-2521
title: 'READ-13: tests/provider.rs docs carry TASK tags and reference the deleted DuckDB backend'
status: Done
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:51'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/tests/provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/provider.rs:mod provider'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/provider.rs:1,12-22,91-100,120-125`

**What**: Two problems. (1) Task provenance: docs open with "TEST-23 / TASK-2195" (narrating that "the previous shape hardcoded the same 15 literals it guarded") and "TEST-5 / TASK-2190 AC #1 + #2" / "AC #3" — acceptance-criteria narration from the finding that drove the tests. (2) Stale docs: the module doc says "the `DuckDB` readback projection" and two test docs discuss "a `DuckDB` whose `coverage_files` table already holds rows" and "the live hazard documented at `extensions/duckdb/src/lib.rs`" — but this crate is the SQLite port and `extensions/duckdb/` no longer exists in the tree. The docs point maintainers at a deleted module and a wrong engine.

**Why it matters**: READ-13: docs go stale on the next change while looking authoritative — exactly what happened here: the DuckDB references survived the SQLite port unread. The task/AC narration is a process artifact on top of the staleness.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Module and test docs in tests/provider.rs name SQLite (the actual backend) and contain no reference to DuckDB or extensions/duckdb
- [x] #2 No test doc opens with a RULE-ID / TASK-XXXX tag or 'AC #N' finding narration

<!-- AC:END -->
