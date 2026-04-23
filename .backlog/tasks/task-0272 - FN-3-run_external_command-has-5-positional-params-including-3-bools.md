---
id: TASK-0272
title: 'FN-3: run_external_command has 5 positional params including 3 bools'
status: To Do
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:25`

**What**: args, dry_run, verbose, tap, raw threaded through 4 nested helpers.

**Why it matters**: Boolean positional args invite swap bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Group into RunOptions struct
- [ ] #2 Thread struct through helpers
<!-- AC:END -->
