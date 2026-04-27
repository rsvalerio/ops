---
id: TASK-0380
title: 'PERF-1: Unnecessary Arc<Value> deep clone on every metadata access'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:38'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:107`

**What**: Metadata::from_context calls (*value).clone() on the cached Arc<serde_json::Value>, doing a full structural deep clone of cargo metadata (which can exceed 1 MB on large workspaces) on every consumer call. The doc-comment claims from_value takes ownership, but the entire Metadata API uses &self.

**Why it matters**: Defeats the purpose of Arc caching; on a workspace with 100 deps, this materially slows about-card rendering.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change Metadata::inner to Arc<serde_json::Value> and from_context to clone the Arc (cheap), not the value
- [ ] #2 Bench shows no regression in unit/integration tests
<!-- AC:END -->
