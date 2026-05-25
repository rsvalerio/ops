---
id: TASK-1551
title: >-
  PERF-3: query_metadata_raw_with_cap issues two DuckDB to_json() round trips
  per call (size probe + payload fetch)
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:27'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:256-283`

**What**: For every `provide_from_db` (i.e. every cached metadata read), the function issues two `query_row` calls:
1. `SELECT octet_length(CAST(to_json(m)::VARCHAR AS BLOB)) FROM metadata_raw m` — runs `to_json` server-side just to measure
2. `SELECT to_json(m)::VARCHAR FROM metadata_raw m` — runs `to_json` again to materialise

DuckDB does not (today) memoise `to_json` between these two prepares, so a workspace with thousands of packages pays the serialisation cost twice per call.

**Why it matters**: SEC-33 / TASK-1194 introduced the size probe specifically to keep an oversized payload from crossing the FFI boundary; the *correct* design pays this twice only when the cap fires. For the common case (payload fits), a single statement can both fetch the bytes AND let Rust check `payload.len()` against the cap before doing anything else — DuckDB's columnar buffer is the same magnitude as the Rust `String`. Alternatives: (a) parameterise the cap in SQL — `SELECT CASE WHEN octet_length(...) > ? THEN NULL ELSE ... END` to push the gating into one round trip; (b) fetch as BLOB and only convert to String when the length check passes (avoids the redundant VARCHAR cast); (c) use `read_json` / `read_json_auto` server-side cap. Today's shape doubles the JSON-serialise cost on the hot path to win RSS on a rare edge case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 query_metadata_raw_with_cap issues a single SQL round trip on the common (under-cap) path
- [ ] #2 Oversized-payload behaviour pinned by query_metadata_raw_rejects_oversized_payload_before_materialising still holds (cap fires before the full payload crosses the FFI boundary)
- [ ] #3 All existing cap tests pass without modification
<!-- AC:END -->
