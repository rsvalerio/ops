---
id: TASK-0569
title: >-
  PERF-3: Python RawTool.uv as Option<toml::Value> deserializes entire [tool.uv]
  table just to test presence
status: Triage
assignee: []
created_date: '2026-04-29 05:04'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:158-161, 183`

**What**: has_tool_uv is computed as raw.tool.as_ref().and_then chained .uv.as_ref().is_some(). RawTool stores uv: Option<toml::Value>, so the entire [tool.uv] subtree (often holding dev-dependencies, sources, indexes) is materialized into an arbitrary nested toml::Value and then thrown away.

**Why it matters**: Wasteful allocation on a parse path called for every about invocation; serde::de::IgnoredAny captures presence with no storage cost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 RawTool.uv becomes Option<serde::de::IgnoredAny> (or equivalent presence-only marker)
- [ ] #2 has_tool_uv semantics unchanged — uv.lock and [tool.uv] continue to flip stack_detail
<!-- AC:END -->
