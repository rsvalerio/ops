---
id: TASK-0672
title: >-
  CONC-3: cargo install/rustup component spawn risks pipe-buffer deadlock under
  captured stdio
status: Done
assignee:
  - TASK-0735
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 06:13'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:63-67, 86-90`

**What**: `Command::spawn()` waited via `wait_timeout` without redirecting stdout/stderr to `Stdio::piped()` or `null()` and without draining child output. Cargo and rustup install can produce large output bursts; if the child fills the inherited pipe buffer (e.g. when running under a captured CI parent) it blocks before the timeout fires, defeating the timeout guarantee.

**Why it matters**: Same family as the deadlock fixed in `has_staged_files_with_timeout` (TASK-0650).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Configure Stdio::inherit() deliberately or pipe + drain stdout/stderr on a thread before wait_timeout
- [x] #2 Document why inheriting stdio is safe here, or switch to Output-based capture
<!-- AC:END -->
