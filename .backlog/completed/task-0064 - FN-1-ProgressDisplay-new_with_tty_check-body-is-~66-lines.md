---
id: TASK-0064
title: 'FN-1: ProgressDisplay::new_with_tty_check body is ~66 lines'
status: Done
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 16:06'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:144`

**What**: Constructor combines TTY detection, theme resolution, multi-progress setup, running-style template assembly, tap-file opening, and struct initialization in a single 66-line body.

**Why it matters**: Constructor mixes policy with plumbing, making it harder to adjust any one concern without touching the rest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract build_running_style(theme) and open_tap_file(path) helpers
- [x] #2 Body under 50 lines
<!-- AC:END -->
