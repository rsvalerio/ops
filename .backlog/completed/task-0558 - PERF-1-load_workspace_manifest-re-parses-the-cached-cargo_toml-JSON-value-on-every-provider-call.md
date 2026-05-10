---
id: TASK-0558
title: >-
  PERF-1: load_workspace_manifest re-parses the cached cargo_toml JSON value on
  every provider call
status: Done
assignee:
  - TASK-0643
created_date: '2026-04-29 05:02'
updated_date: '2026-04-29 14:26'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:44-60`

**What**: When cargo_toml is cached on the Context, the function does (**cached).clone() (deep-clones a serde_json::Value) then serde_json::from_value::<CargoToml>(...). This runs once per about provider (identity, units, coverage, deps_provider) — i.e. the cached manifest is JSON-cloned and re-deserialized 4x per ops about invocation despite the explicit cache.

**Why it matters**: The cache only avoids the fs read, not the parse; on real workspaces the redundant serde_json::Value clone + reparse dominates load_workspace_manifest cost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cache the typed CargoToml (e.g. via Arc<CargoToml>) instead of re-deserializing from serde_json::Value
- [x] #2 Confirm ctx.refresh = true still invalidates correctly
<!-- AC:END -->
