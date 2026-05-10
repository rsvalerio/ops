---
id: TASK-0374
title: 'SEC-13: which shell-out for binary detection is non-portable and slow'
status: Done
assignee:
  - TASK-0419
created_date: '2026-04-26 09:38'
updated_date: '2026-04-27 10:58'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:53`

**What**: check_binary_installed shells out to which, which is not present on Windows and pays subprocess overhead for every probe.

**Why it matters**: Probe is called per-tool per-status-check; on Windows it returns a misleading "not installed" for every tool.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace Command::new(which) with the which crate or PATH-walking helper
- [x] #2 Cross-platform tests verify detection on macOS/Linux and a documented fallback for Windows
<!-- AC:END -->
