---
id: TASK-0243
title: >-
  PERF-3: merge_indexmap clones every overlay key and value even when overlay is
  owned
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 14:32'
labels:
  - rust-code-review
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:17`

**What**: Iterates overlay by ref and clones both K and V into base; on large command tables this multiplies allocations.

**Why it matters**: Hot path during config load; minor but avoidable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Accept IndexMap<K,V> by value when overlay is owned
- [ ] #2 Use base.extend(overlay.into_iter()) when ownership is available
<!-- AC:END -->
