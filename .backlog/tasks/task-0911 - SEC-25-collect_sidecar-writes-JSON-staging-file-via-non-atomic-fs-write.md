---
id: TASK-0911
title: 'SEC-25: collect_sidecar writes JSON staging file via non-atomic fs::write'
status: Triage
assignee: []
created_date: '2026-05-02 10:11'
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
- [ ] #1 JSON file is written via ops_core::config::atomic_write so a crash leaves either the previous file intact or the new file fully populated
- [ ] #2 Test asserts no leftover .tmp sibling remains after a successful collect
<!-- AC:END -->
