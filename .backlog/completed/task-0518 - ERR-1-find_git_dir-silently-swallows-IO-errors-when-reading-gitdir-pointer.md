---
id: TASK-0518
title: 'ERR-1: find_git_dir silently swallows IO errors when reading gitdir pointer'
status: Done
assignee:
  - TASK-0535
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 13:56'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:67`

**What**: `read_to_string(file).ok()?` discards any error reading the `.git` pointer file (EACCES, EISDIR, IO error during read). The walker then continues upward as if `.git` did not exist.

**Why it matters**: A user with a permissions issue or a partially-written .git file gets a silent fallback to ancestor .git (or no repo found), with no diagnostic. Unlike SEC-14 cases, IO errors here have legitimate-config root causes worth surfacing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log read errors at tracing::debug! before falling through
- [x] #2 Test that a non-readable .git pointer is logged
<!-- AC:END -->
