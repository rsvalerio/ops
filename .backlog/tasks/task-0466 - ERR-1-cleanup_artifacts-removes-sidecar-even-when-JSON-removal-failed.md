---
id: TASK-0466
title: 'ERR-1: cleanup_artifacts removes sidecar even when JSON removal failed'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 05:46'
updated_date: '2026-04-28 18:50'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:194`

**What**: SidecarIngestorConfig::cleanup_artifacts warns when remove_file(json_path) fails, then unconditionally calls remove_workspace_sidecar(...). The JSON file holds the sidecar's data lineage and is the input to checksum_file on retry; leaving it on disk while removing the sidecar inverts the documented "leftover staged JSON or sidecar is a recoverable disk-hygiene issue" invariant.

**Why it matters**: On the next run, JSON is present but sidecar is gone, so read_workspace_sidecar fails before checksum is recomputed — making a partial-failure state unrecoverable without manual intervention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reorder cleanup so the sidecar is removed only after the JSON removal has succeeded (or atomically; an Err-and-leave-both path is fine)
- [ ] #2 New test creates a json_path that cannot be removed (e.g. parent dir lacks write perms) and asserts the sidecar still exists afterwards
<!-- AC:END -->
