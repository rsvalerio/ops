---
id: TASK-1133
title: >-
  READ-5: merge_indexmap allocates formatted-key Vec even when no overlay
  collisions exist
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 07:40'
updated_date: '2026-05-09 10:55'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:32`

**What**: `merge_indexmap` builds `replaced: Vec<String>` by iterating overlay keys, formatting each via `format!(\"{k:?}\")`, and only then checks `if !replaced.is_empty()`. The Vec is allocated unconditionally before the empty check.

**Why it matters**: Sits on the layered-config-load hot path (called once per overlay file plus once for env vars). Common no-collision case pays pure overhead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Short-circuit when base.contains_key finds no collisions; skip Vec allocation entirely on no-overlap path
- [x] #2 Preserve the SEC-21 / TASK-0745 escape contract via the existing test
<!-- AC:END -->
