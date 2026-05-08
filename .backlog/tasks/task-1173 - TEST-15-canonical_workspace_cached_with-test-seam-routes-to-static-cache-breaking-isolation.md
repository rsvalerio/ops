---
id: TASK-1173
title: >-
  TEST-15: canonical_workspace_cached_with test seam routes to static cache,
  breaking isolation
status: To Do
assignee:
  - TASK-1266
created_date: '2026-05-08 08:07'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:204`

**What**: The `#[cfg(test)]` seam `canonical_workspace_cached_with` forwards to `default_workspace_cache()` (the process-global `OnceLock`) — yet its doc comment claims it "forwards to a fresh local cache to keep test isolation". The companion test (line 802 `canonical_workspace_cached_collapses_burst_to_single_canonicalize`) keys by `process::id()`+nanos to dodge the leak, but any two tests in the same binary that key on the same path now share state.

**Why it matters**: The doc/code disagreement means a future test author following the comment will write tests that silently leak state into the static cache. The "collapses burst" test depends on the static cache's behaviour rather than a fresh instance — a regression that broke the runner-scoped cache could still pass.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test seam constructs a fresh WorkspaceCanonicalCache per call (or threads one through), matching its docstring.
- [ ] #2 The burst-startup test exercises a fresh local cache instance.
<!-- AC:END -->
