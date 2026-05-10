---
id: TASK-0946
title: >-
  READ-5: extensions-rust about query workspace glob uses to_string_lossy on
  member relpaths, lossily collapsing non-UTF-8 bytes
status: Done
assignee: []
created_date: '2026-05-02 16:03'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:262`

**What**: `resolved.push(rel.to_string_lossy().to_string())` lossily collapses any non-UTF-8 bytes in a workspace-relative member path to U+FFFD before storing for downstream dedup/lookups. Two distinct members differing only in non-UTF-8 bytes alias to the same key.

**Why it matters**: Same READ-5 pattern as TASK-0900 (`resolve_spec_cwd`) which was fixed by skipping non-UTF-8 paths with a `tracing::warn!`. This Rust-side workspace glob walk has the same flaw and should adopt the same policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Skip non-UTF-8 member paths with a tracing::warn! (matching the resolve_spec_cwd post-fix policy from TASK-0900) instead of lossy-converting
- [x] #2 Regression test creates a member path with non-UTF-8 bytes and asserts it is logged + skipped, not silently aliased
<!-- AC:END -->
