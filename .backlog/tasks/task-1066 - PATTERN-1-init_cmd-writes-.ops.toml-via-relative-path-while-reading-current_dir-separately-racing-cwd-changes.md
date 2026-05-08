---
id: TASK-1066
title: >-
  PATTERN-1: init_cmd writes .ops.toml via relative path while reading
  current_dir separately, racing cwd changes
status: Done
assignee: []
created_date: '2026-05-07 21:18'
updated_date: '2026-05-08 06:29'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:21-24`

**What**: `let path = PathBuf::from(\".ops.toml\")` is used for `OpenOptions::create_new` and parent fsync, while `cwd = std::env::current_dir()?` is read independently. Two filesystem operations against cwd race if cwd changes mid-call (signal handler, threaded init template).

**Why it matters**: Small TOCTOU plus surprising parent-fsync logic that maps the empty parent component to `Path::new(\".\")`. Capturing cwd once and joining produces an unambiguous absolute path for both the create and the fsync.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Capture cwd once and join with .ops.toml; use the absolute path for both creation and parent fsync
- [x] #2 Regression test exercises a relative-cwd test fixture and verifies the file lands in the captured directory
<!-- AC:END -->
