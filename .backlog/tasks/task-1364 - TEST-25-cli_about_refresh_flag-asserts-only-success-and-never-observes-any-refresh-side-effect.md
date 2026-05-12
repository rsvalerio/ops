---
id: TASK-1364
title: >-
  TEST-25: cli_about_refresh_flag asserts only success() and never observes any
  refresh side-effect
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 23:34'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/tests/integration.rs:642`

**What**: The test runs `ops about --refresh` and asserts only `.success()`. The test name advertises the `--refresh` semantics but the body verifies nothing refresh-specific (no stdout/stderr marker, no on-disk mutation check, no cache-mtime probe).

**Why it matters**: TEST-25: a regression that silently ignored `--refresh` and exited 0 still passes. The test is functionally equivalent to `cli_about_runs` without the flag.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Assert a refresh-specific stdout/stderr marker (e.g. refresh banner / debug line) or an observable mutation (e.g. cache mtime change) on the file --refresh is meant to recompute
- [ ] #2 If --refresh is a no-op when no [about] block exists, seed .ops.toml in the test so the live refresh path is exercised
<!-- AC:END -->
