---
id: TASK-2900
title: 'runner: terminal close (SIGHUP) no longer reaches the runner during a raw step''s foreground hand-off'
status: Done
assignee: []
created_date: '2026-11-20 22:05'
updated_date: '2026-11-21 07:38'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2902'
modified_files:
  - crates/runner/src/command/process_group.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/process_group.rs:658` (the relay in `spawn_and_wait_blocking`)

**What**: With the raw-step foreground hand-off, a terminal hangup delivers SIGHUP to the foreground group — the step's group — only. The runner itself no longer receives it, so after a terminal close the runner survives headless and runs the remaining plan to completion (steps fail on the dead tty), where the previous shared-group design died with the terminal. The Ctrl-C relay re-raises SIGINT only; SIGHUP is deliberately not relayed because a targeted `kill -HUP <step pid>` would then also kill the run.

**Why it matters**: Behaviour delta on terminal close: a detached `ops` keeps consuming resources (builds, locks) after the user closed the session. Bounded and self-limiting (the plan finishes or fails), but a deliberate choice between "relay SIGHUP like SIGINT" and "document surviving" has not been made.

**Origin**: discovered during TASK-2898 while fixing TASK-2866 (side effect of the foreground hand-off).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A decision is recorded (relay SIGHUP or document surviving) and implemented or noted in docs/configuration.md

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision (user, 2026-11-21): relay SIGHUP onto the runner like SIGINT.

Fixed in 8a19921d: the raw-step relay now re-raises SIGHUP as well as SIGINT on the runner when the foreground step died of it, so closing the terminal ends the run as it did before the hand-off. Recorded in docs/configuration.md, including the trade-off (kill -INT/-HUP aimed at the step also ends the run) and that a step catching SIGHUP keeps the run alive. New pty test terminal_hangup_on_the_pty_still_ends_the_run fails on the old code (runner exits 0). Pre-merge and integration ops verify 16/16; nextest ops-runner 309/309.

<!-- SECTION:NOTES:END -->
