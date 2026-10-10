---
id: TASK-2598
title: 'VER-4: Clear poison after recovery in provide_via_ingestor'
status: To Do
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - ver
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
priority: low
ordinal: 1000
dedup_key: 'VER-4:extensions/sqlite/src/sql/ingest/orchestrator.rs:provide_via_ingestor'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/orchestrator.rs:90-99`

**What**: `provide_via_ingestor` recovers a poisoned per-table ingest mutex the same way as the registry lock: a comment (~line 92) explains the mutex only guards a `()`, then `poisoned.into_inner()` (~line 98) reclaims the guard — but `clear_poison` is never called on that per-table mutex.

**Why it matters**: VER-4: `Mutex::clear_poison` is stable since 1.77 (workspace MSRV 1.97) and belongs on exactly this recovery path — the recovery re-establishes the (trivial) invariant, so the poison flag should be dropped. Left set, every later ingest for that table re-enters the recovery branch and re-emits its `tracing::warn!` breadcrumb, turning one transient panic into per-ingest log spam. Fix: clear the poison on the recovered per-table mutex before running the pipeline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The per-table mutex poison recovery in provide_via_ingestor clears the poison flag, and a test pins that a poisoned-then-recovered table does not warn on the next ingest
<!-- AC:END -->
