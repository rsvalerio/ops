---
id: TASK-0146
title: 'SEC-25: install_hook check-then-write on hook_path is racy (TOCTOU)'
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:40'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:62`

**What**: `install_hook` calls `hook_path.exists()`, then if existing matches reads/compares and returns, else `std::fs::write(&hook_path, ...)` at line 86. Between the `exists()` check (and the `read_to_string`) and the `write`, another process can install a different pre-commit hook; the ops hook will then silently overwrite it even though the earlier check would have tripped the "not installed by ops" bail.

**Why it matters**: Hooks live in `.git/hooks/`. Install commands are often run concurrently with other tooling (other `ops install`, husky, lefthook) during repo bootstrap. A race here can blow away a non-ops hook that was written after the check. Fix: use `OpenOptions::new().write(true).create_new(true).open(path)` for the write path, and only fall back to the read+overwrite branch when `create_new` returns `AlreadyExists`. That narrows the race to the legacy-update branch, where it is still accepted behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace check-then-write with create_new for the fresh-install path
- [x] #2 Document the accepted race window for the legacy-marker overwrite path
<!-- AC:END -->
