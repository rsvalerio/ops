---
id: TASK-1132
title: >-
  PERF-3: Context::get_or_provide allocates key.to_string() twice on every cache
  miss
status: Done
assignee:
  - TASK-1262
created_date: '2026-05-08 07:40'
updated_date: '2026-05-08 15:33'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:380`

**What**: `Context::get_or_provide` takes `key: &str` and on a cache miss converts to `String` twice — once for `in_flight.insert(key.to_string())` (line 380) and again for `data_cache.insert(key.to_string(), ...)` (line 388).

**Why it matters**: Data providers are queried repeatedly during `ops about`, `ops data`, and the runner's `query_data` path. Easy to remove with a single owned `key`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Allocate key.to_string() once and reuse for both in_flight.insert and data_cache.insert
- [ ] #2 Cycle error path returns the same string content
<!-- AC:END -->
