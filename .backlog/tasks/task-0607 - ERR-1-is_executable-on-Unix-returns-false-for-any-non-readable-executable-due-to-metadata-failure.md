---
id: TASK-0607
title: >-
  ERR-1: is_executable on Unix returns false for any non-readable executable due
  to metadata failure
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:20'
updated_date: '2026-04-29 10:43'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:132`

**What**: find_on_path is the SEC-13-rationalised replacement for `which`. On Unix, is_executable calls std::fs::metadata(path) (follows symlinks) and treats any Err as "not executable". A directory in PATH the user can`t read but a target binary that is executable for them is rejected. More commonly, broken symlinks in PATH (nix-env update mid-run) silently fail probing.

**Why it matters**: Probe failures cascade into "tool not installed" / re-install attempts. SEC-13 task that replaced `which` should at least not regress vs `which` symlink handling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Distinguish 'metadata error' from 'exists but not executable' so probe consumers can decide
- [ ] #2 Test covers a broken-symlink-on-PATH case
<!-- AC:END -->
