---
id: TASK-0257
title: >-
  READ-5: write_workspace_sidecar round-trips working_directory via
  to_string_lossy bytes
status: To Do
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:119`

**What**: Non-UTF-8 working_directory is replaced with U+FFFD, so upsert_data_source later associates data with a path that cannot locate the actual workspace.

**Why it matters**: Sidecar path corruption silently on non-UTF-8 paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use working_directory.as_os_str().as_encoded_bytes() (or fail for non-UTF-8)
- [ ] #2 Test with non-UTF-8 PathBuf
<!-- AC:END -->
