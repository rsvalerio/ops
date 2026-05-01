---
id: TASK-0255
title: >-
  FN-1: SidecarIngestorConfig::load_with_sidecar mixes init, exec, count, IO,
  checksum, upsert, cleanup
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 09:14'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:61`

**What**: Multiple abstraction levels interleaved with guard-holding FS IO in ~50-line body.

**Why it matters**: Hard to reason about ordering and failure semantics; contributed to the other defects already filed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Split into create_tables, count_records, persist_record, cleanup_artifacts helpers
- [x] #2 Each helper independently testable
<!-- AC:END -->
