---
id: TASK-1414
title: >-
  PERF-3: merge_env_vars eagerly collects ops_keys Vec even when env_config
  build succeeds
status: Done
assignee:
  - TASK-1454
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 21:48'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:113`

**What**: `merge_env_vars` collects every `OPS__`-prefixed env key into a `Vec<String>` up front so keys can be embedded in error context strings. On the happy path this Vec is allocated, sorted, and dropped unused. Every `ops <cmd>` invocation walks every env var with `vars_os` and `into_string` to do so.

**Why it matters**: The Vec is only ever read by the two `with_context` closures — both of which only fire on error. Compute the key list lazily inside the closures (or defer via `LazyCell`) so the success path skips the allocation. CLI startup is on the critical path of every command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 defer ops_keys collection so the success path does not allocate the Vec
- [x] #2 error paths still produce the same keys: [...] context formatting
- [x] #3 regression test pins that the no-OPS__-env shortcut path remains allocation-free for the keys Vec
<!-- AC:END -->
