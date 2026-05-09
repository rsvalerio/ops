---
id: TASK-1129
title: >-
  READ-5: ops_toml_max_bytes re-reads env per call, inconsistent with
  manifest_max_bytes OnceLock cache
status: Done
assignee:
  - TASK-1262
created_date: '2026-05-08 07:39'
updated_date: '2026-05-08 15:35'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:42`

**What**: `ops_toml_max_bytes()` parses `OPS_TOML_MAX_BYTES` from `std::env::var` on every invocation, while sibling `manifest_max_bytes()` at `crates/core/src/text.rs:63` caches via `OnceLock<u64>` and explicitly documents that mid-run env changes are NOT observed. Both caps are reached through similar paths (`read_capped_toml_file` is called for `.ops.toml`, every `.ops.d/*.toml`, and global config) so the divergent observability semantics are surprising and there is no warn-on-unparseable arm.

**Why it matters**: Operators reading the documented contract on `manifest_max_bytes` will be surprised the symmetric env var is observed mid-run, and unparseable values get silently ignored.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the resolved cap behind a OnceLock<u64> matching manifest_max_bytes shape including warn-on-unparseable
- [ ] #2 Document process-lifetime semantics on ops_toml_max_bytes
<!-- AC:END -->
