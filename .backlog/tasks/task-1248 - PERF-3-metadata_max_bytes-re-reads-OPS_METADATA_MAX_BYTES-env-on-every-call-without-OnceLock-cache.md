---
id: TASK-1248
title: >-
  PERF-3: metadata_max_bytes re-reads OPS_METADATA_MAX_BYTES env on every call
  without OnceLock cache
status: Done
assignee:
  - TASK-1262
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 15:43'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:47`

**What**: `metadata_max_bytes()` re-parses the env var on each invocation; `query_metadata_raw_with_cap` calls it for every `provide_from_db`. Mirrors the PERF-3 / TASK-1129 finding for `ops_toml_max_bytes` and is inconsistent with the `manifest_max_bytes` OnceLock cache.

**Why it matters**: Avoidable env syscall plus String alloc on the about/data-source hot path; inconsistent caching policy across sibling cap helpers makes future audits noisier.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Resolve cap once into a OnceLock<u64> populated on first call
- [ ] #2 Match the ops_core::text manifest_max_bytes cache pattern
- [ ] #3 Regression test pinning two consecutive calls yield the snapshotted value
<!-- AC:END -->
