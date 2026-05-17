---
id: TASK-1474
title: >-
  ERR-1: expand.rs test-helpers swallow Mutex poison and return a count without
  surfacing it
status: Done
assignee:
  - TASK-1480
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:57'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:178-185,265-272`

**What**: Both `#[cfg(test)]` accessors swallow `PoisonError` via `unwrap_or_else(|e| e.into_inner().map.len())` and silently report a length — a test that asserts cache size after another thread panicked will observe a "successful" count instead of the panic surface.

**Why it matters**: The whole point of test seams is to surface state; swallowing poison here makes a flake (a panicked sibling that left the cache poisoned) look like a value-mismatch failure two tests later. ERR-1 says handle-or-propagate, never swallow without context.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both helpers should clear_poison() and emit a tracing::warn! (or eprintln! in test code) noting that poison was recovered, then return the count
- [ ] #2 Add a regression test that panics inside cached_ops_root_arc and asserts the helpers report the poison breadcrumb
<!-- AC:END -->
