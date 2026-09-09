---
id: TASK-2125
title: 'READ-12: two ops-duckdb log sites interpolate values into the message instead of emitting them as structured fields'
status: To Do
assignee: []
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions/duckdb/src/sql/mod.rs
  - extensions/duckdb/src/sql/ingest/sidecar.rs
priority: low
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/mod.rs:62`

**What**: candidate list (all non-test):

* `extensions/duckdb/src/sql/mod.rs:62-68` — `query_or_warn` emits
  `tracing::error!(query = label, "duckdb query failed (hard); {degraded}: {e:#}")`
  and the matching `warn!`. `query` is a proper field, but the degraded-mode
  description and the whole error chain are baked into the message string, so
  a JSON subscriber cannot filter or group on either. `error = %e` /
  `degraded = %degraded` (or `error = ?e`) are the structured forms.
* `extensions/duckdb/src/sql/ingest/sidecar.rs:127-130` —
  `tracing::warn!("remove_workspace_sidecar({}): {e}", dir.entry_path(&entry).display())`
  is fully positional: no fields at all, and the message itself varies per
  call, so the event has no stable name to aggregate on. The sibling cleanup
  breadcrumbs in `ingestor.rs:373` and `ingestor.rs:413/424` already use the
  `source = …, paths = %…, error = ?…` field shape; this one did not follow.

**Why it matters**: the crate is inconsistent with itself — `ingestor.rs`,
`orchestrator.rs`, `helpers.rs` and `deps.rs` all emit named fields, and these
two sites are the exceptions. Positional interpolation makes the message the
only searchable surface, so an operator cannot filter on the failing path or
the error kind, and every distinct path produces a distinct event name.

<!-- scan confidence: candidates to inspect; both verified by reading, test code excluded -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 query_or_warn emits the degraded description and the error as named fields, leaving a constant message
- [ ] #2 remove_workspace_sidecar emits the sidecar path and the error as named fields with a constant message, matching the ingestor.rs breadcrumb shape
- [ ] #3 no existing test that asserts on captured log text regresses (see poison_recovery_emits_warn_log for the capture_warn pattern in use)
<!-- AC:END -->
