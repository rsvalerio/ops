---
id: TASK-2870
title: 'CL-3: sidecar ingestors run their expensive collect before the workspace path is checked for sidecar compatibility'
status: To Do
assignee: []
created_date: '2026-11-18 21:47'
updated_date: '2026-11-20 13:26'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2893'
modified_files:
  - extensions/sqlite/src/sql/ingest/sidecar.rs
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/sidecar.rs` (`write_workspace_sidecar`), `extensions/sqlite/src/ingestor.rs` (`SidecarIngestorConfig::collect_sidecar`), `extensions/sqlite/src/sql/ingest/orchestrator.rs` (`provide_via_ingestor`)

**What**: `write_workspace_sidecar` now refuses a workspace path that `read_workspace_sidecar` would reject (a control byte, or non-UTF-8 off Unix), naming the path. But the sidecar ingestors (tokei, rust-loc, test-coverage) only call `collect_sidecar` after their external tool run has finished. A workspace whose path holds such a byte therefore still pays the full tool run (for test-coverage, an instrumented test run) before the refusal, and since nothing gets loaded, every later invocation repeats the run and fails the same way.

**Why it matters**: A deterministic failure is detected after the costly step, on every run. Checking the path before collect (in `provide_via_ingestor` for sidecar ingestors, or a `DataIngestor` pre-check hook) would fail fast.

**Origin**: discovered during TASK-2853 while fixing TASK-2833.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A workspace path the sidecar format cannot carry is refused before any sidecar ingestor's collect runs its external tool
<!-- AC:END -->
