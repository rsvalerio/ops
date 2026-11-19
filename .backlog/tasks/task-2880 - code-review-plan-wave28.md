---
id: TASK-2880
title: 'code-review-plan-wave28'
status: In Progress
assignee: []
created_date: '2026-11-19 07:57'
updated_date: '2026-11-19 17:17'
labels:
  - code-review-wave
dependencies:
  - TASK-2866
  - TASK-2867
modified_files:
  - crates/core/src/subprocess/mod.rs
  - crates/runner/src/command/exec.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave28
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
child-process lifecycle follow-ups: raw-mode process-tree teardown and spawn serialization beyond the runner
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none

Branch: code-review/TASK-2880
Worktree: /home/rsvalerio/projects/.wave-TASK-2880

Parked: runner stopped by the weekly usage limit (resets 2026-11-21 01:00 Europe/Amsterdam); code committed at 636e9f5c, pre-merge verify state unknown, not merged
Resume from branch code-review/TASK-2880, worktree /home/rsvalerio/projects/.wave-TASK-2880

Parked: Merge not landed: integration ops verify failed with ENOSPC (disk 100% full, 0 bytes free) while compiling ops-core. This is an environment fault, not a code failure. Pre-merge verify (9/9) and nextest (1742/1742, after rebasing onto 68ac5ce5) were green. Retry the locked merge once disk is freed. TASK-2866 is still open: the foreground tcsetpgrp hand-off was not done (job control, background runs, the SIGINT relay and a pty harness are needed; see its notes) and it is back in To Do for re-triage. TASK-2867 is still open: AC#1 is done, AC#2 needs a macOS run.
Resume from branch code-review/TASK-2880, worktree /home/rsvalerio/projects/.wave-TASK-2880

<!-- SECTION:NOTES:END -->
