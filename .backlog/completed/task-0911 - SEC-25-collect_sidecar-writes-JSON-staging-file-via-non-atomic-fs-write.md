---
id: TASK-0911
title: 'SEC-25: collect_sidecar writes JSON staging file via non-atomic fs::write'
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 14:54'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:81`

**What**: collect_sidecar calls std::fs::write directly on the JSON staging file then routes the workspace sidecar through atomic_write. A crash between the JSON write returning and the inode flushing leaves a torn or zero-byte JSON file that load_with_sidecar will subsequently feed to read_json_auto. TASK-0663 fixed the workspace sidecar but left the JSON path on the bare write.

**Why it matters**: Tokei/coverage ingest can corrupt the database with truncated JSON after a power loss or kill mid-collect.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 JSON file is written via ops_core::config::atomic_write so a crash leaves either the previous file intact or the new file fully populated
- [x] #2 Test asserts no leftover .tmp sibling remains after a successful collect
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
collect_sidecar now writes the JSON staging file via ops_core::config::atomic_write (sibling temp + fsync + rename), matching the workspace-sidecar path that TASK-0663 already hardened. A crash mid-collect now leaves either the previous JSON intact or the new payload fully populated — never the torn/zero-byte file that would corrupt read_json_auto. Added collect_sidecar_writes_json_atomically_no_tmp_leftover test asserting no .tmp.* sibling lingers after a successful collect.
<!-- SECTION:NOTES:END -->
