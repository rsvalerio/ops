---
id: TASK-1462
title: >-
  ERR-1: cached_ops_root_arc renders OPS_ROOT via lossy Path::display, defeating
  strict-expand contract
status: Done
assignee:
  - TASK-1481
created_date: '2026-05-15 18:50'
updated_date: '2026-05-17 08:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:164-167`

**What**: `cached_ops_root_arc` builds the cached `OPS_ROOT` substitution from `owned.display().to_string()`. `Path::display()` substitutes `U+FFFD` for non-UTF-8 bytes, so a workspace path with a non-UTF-8 component silently flows into `Variables::builtins["OPS_ROOT"]`. That value is consumed by `try_expand`, which is supposed to fail loudly on bad inputs but here receives a lossy-renamed string with no error.

**Why it matters**: The strict-expand contract (TASK-0450) is the only thing preventing argv/cwd/env values from being silently corrupted. A non-UTF-8 workspace root will spawn subprocesses into the wrong directory or substitute the wrong path into command args, with no diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cached_ops_root_arc returns a Result and from_env propagates a typed ExpandError when the canonicalized/raw OPS_ROOT path is not valid UTF-8
- [ ] #2 New #[cfg(unix)] test constructs a non-UTF-8 PathBuf via OsStr::from_bytes and asserts from_env fails rather than producing a U+FFFD-substituted OPS_ROOT
<!-- AC:END -->
