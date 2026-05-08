---
id: TASK-1194
title: >-
  SEC-33: query_metadata_raw_with_cap enforces byte cap after materialising the
  full DuckDB row
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 08:14'
updated_date: '2026-05-08 14:06'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:236-260`

**What**: The OOM guard advertised by METADATA_MAX_BYTES_DEFAULT and OPS_METADATA_MAX_BYTES runs only after `conn.query_row("SELECT to_json(m)::VARCHAR FROM metadata_raw m", ...)` has already materialised the full payload into json_text: String. By the time `if len > cap` is checked, the very allocation the cap is meant to prevent has already happened — and DuckDB's columnar buffer is still live, so peak RSS is at least 2× the payload before the bail fires.

**Why it matters**: The TASK-1034 doc-comment justifies the cap as "fail with a clear error before the OS kills the process", but the OS will kill the process during query_row for a pathological payload, never reaching the check.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A metadata_raw row whose JSON length exceeds the configured cap surfaces the byte-cap error without ever materialising the full text into a Rust String — verified by a test that wires up a 100-MiB synthetic payload with a 1-MiB cap.
- [x] #2 The error message and warn line still cite the observed length and the override env var, matching today's output.
<!-- AC:END -->
