---
id: TASK-1018
title: >-
  ERR-1: terraform/about count_local_modules silently swallows read_dir errors
  and per-entry IO failures
status: Done
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-07 23:11'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:283-298`

**What**: `count_local_modules` opens `<root>/modules` with `let Ok(entries) = std::fs::read_dir(&modules_dir) else { return None; };` then iterates with `entries.flatten()`. Both swallow IO failures:

- A non-NotFound failure on `read_dir` (EACCES on `modules`, IsADirectory on a symlink loop, EIO) is collapsed to "no modules" with no log. Sister-parser sites in this same crate (`find_required_version`, lib.rs:99-105) explicitly distinguish NotFound from real IO failures with a `tracing::warn!`.
- `entries.flatten()` then drops any per-entry `read_dir` iterator error (transient EIO, name-decode failure on a removable filesystem) silently — TASK-0935 / TASK-0942 already pinned this pattern as ERR-1 elsewhere; this site was missed.

**Why it matters**: an operator who can't read `modules/` (permission boundary, NFS hiccup) sees a misleadingly empty `module_count` in the About card and has no signal in the log to investigate. The fix is one-liner symmetry with `find_required_version`: match on the error kind, warn on non-NotFound, and replace `.flatten()` with `.filter_map(|e| match e { Ok(e) => Some(e), Err(err) => { tracing::warn!(…); None } })` (or use the `read_dir` helper sister parsers already use).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Distinguish NotFound from real read_dir errors and emit tracing::warn! on the non-NotFound path, mirroring find_required_version
- [ ] #2 Replace entries.flatten() with a filter_map that logs per-entry IO failures
<!-- AC:END -->
