---
id: TASK-0663
title: >-
  SEC-25: write_workspace_sidecar uses bare fs::write with no fsync or atomic
  rename
status: To Do
assignee:
  - TASK-0739
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:148-159`

**What**: Sidecar write uses bare `std::fs::write` with no `sync_all`/atomic-rename, unlike the hook installer's `write_temp_hook` (extensions/hook-common/src/install.rs:140-152) which fsyncs.

**Why it matters**: A crash between `collect_sidecar` and `load_with_sidecar` can leave a zero-byte or torn sidecar; `read_workspace_sidecar` will then surface it as the workspace_root, which is later interpolated into `upsert_data_source`. Combined with the staged JSON staying on disk, retry produces a row with an empty/garbled workspace_root instead of failing fast.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Stage to <sidecar>.tmp, sync_all, then rename over the destination (matching install::write_temp_hook)
- [ ] #2 Add a test asserting that a sidecar truncated to zero length is rejected by read_workspace_sidecar (or simply cannot be produced by a successful write_workspace_sidecar after fault injection)
<!-- AC:END -->
