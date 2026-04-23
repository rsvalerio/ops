---
id: TASK-0283
title: 'ERR-4: prompt_hook_install invokes child with .status() and swallows stderr'
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 07:49'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:81`

**What**: Spawn error or non-zero status surfaces as bare ExitCode::FAILURE.

**Why it matters**: User cannot see why re-exec failed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Capture child stderr on failure
- [x] #2 Include in anyhow context
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Obsolete after TASK-0174: prompt_hook_install no longer re-execs current_exe(). It now dispatches in-process via run_before_{commit,push}_install() whose anyhow::Result<()> is propagated with full source chain via ?, satisfying the intent of capturing failure detail.
<!-- SECTION:NOTES:END -->
