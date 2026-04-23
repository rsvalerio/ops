---
id: TASK-0156
title: >-
  TEST-5: duckdb sidecar helpers (write/read/remove_workspace_sidecar, io_err)
  lack direct tests
status: Done
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 08:40'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:104-127`

**What**: `write_workspace_sidecar`, `read_workspace_sidecar`, and `remove_workspace_sidecar` are part of the public `sql` API and drive the `SidecarIngestorConfig` flow, but there are no unit tests for them in `ingest.rs`. They are exercised only indirectly via integration-like paths.

**Why it matters**: TEST-5 — public API functions need at least one test. Regressions in filename conventions or encoding (UTF-8 via `to_string_lossy`) go undetected. Quick round-trip tests with `tempfile::tempdir()` would close the gap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 round-trip write/read/remove tests added using tempdir
- [x] #2 tests cover sidecar filename derivation from name parameter
<!-- AC:END -->
