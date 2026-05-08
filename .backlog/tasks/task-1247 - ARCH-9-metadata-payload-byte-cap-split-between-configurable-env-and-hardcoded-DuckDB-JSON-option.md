---
id: TASK-1247
title: >-
  ARCH-9: metadata payload byte cap split between configurable env and hardcoded
  DuckDB JSON option
status: To Do
assignee:
  - TASK-1262
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/views.rs:46` and `extensions-rust/metadata/src/lib.rs:40`

**What**: `metadata_max_bytes()` reads OPS_METADATA_MAX_BYTES (default 64 MiB) and enforces it on the post-DuckDB read in `query_metadata_raw_with_cap`, but `metadata_raw_create_sql` hardcodes `maximum_object_size=67108864` on the DuckDB JSON ingestor. Operators raising the env-knob still hard-fail inside DuckDB during ingest with a less actionable error.

**Why it matters**: Two sources of truth for the same byte cap silently invert the operator's override; lowering the env-knob does not save memory because DuckDB still buffers up to 64 MiB during ingest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Thread metadata_max_bytes() into metadata_raw_create_sql so both paths share one knob
- [ ] #2 Reject zero/non-numeric env values consistently across both surfaces
- [ ] #3 Test pinning that raising OPS_METADATA_MAX_BYTES propagates into the DuckDB CREATE TABLE option
<!-- AC:END -->
