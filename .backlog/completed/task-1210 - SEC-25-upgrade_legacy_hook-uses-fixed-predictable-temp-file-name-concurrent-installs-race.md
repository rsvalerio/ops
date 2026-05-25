---
id: TASK-1210
title: >-
  SEC-25: upgrade_legacy_hook uses fixed predictable temp-file name; concurrent
  installs race
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 08:19'
updated_date: '2026-05-08 14:16'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:144-184`

**What**: upgrade_legacy_hook builds a fixed sibling temp path .{file_name}.ops-tmp and creates it with OpenOptions::create_new. TASK-1113 closed the single-process stale-leftover case. Concurrent installs (two ops install runs against shared worktrees) still race: process A creates the temp, process B sees AlreadyExists, removes process A's mid-write temp file, and writes its own.

**Why it matters**: Install paths run on developer machines where multiple checkouts can share a .git/worktrees/<name>/hooks/ directory. Standard hardening is mkstemp-style randomised suffixes so two concurrent writers cannot collide on the temp name.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 write_temp_hook switches to a tempfile::NamedTempFile::new_in(parent)-style randomised path; the rename target stays the canonical hook path.
- [x] #2 A regression test simulates two concurrent upgrade_legacy_hook calls against the same hook_path and asserts that exactly one rename wins, the other returns a typed error.
<!-- AC:END -->
