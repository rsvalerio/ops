---
id: TASK-1201
title: >-
  PERF-3: load_workspace_manifest deep-clones serde_json Value before
  deserialising it back to CargoToml
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 08:16'
updated_date: '2026-05-09 11:12'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:237-243`

**What**: When the typed cache misses but the JSON cache hits, the loader runs `(**cached).clone()` — a deep clone of the full serde_json::Value tree representing the entire Cargo.toml — and then immediately consumes that clone in serde_json::from_value. The clone is pure waste: the Value is owned-by-deserialise and never reused.

**Why it matters**: This sits on the hot path of every ops about invocation that has a JSON-cache hit but typed-cache miss (after ctx.refresh = true, after LRU eviction, or in a daemon visiting many workspaces). Each clone allocates one Box per nested map / array node — a multi-MB workspace easily clones 10k+ allocations only to drop them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 load_workspace_manifest no longer calls (**cached).clone(); it either deserialises through Value::deserialize against a borrowed reference, or the JSON bridge is removed and the typed value is cached directly.
- [x] #2 Existing tests for Arc::ptr_eq / refresh / cross-thread sharing continue to pass.
<!-- AC:END -->
