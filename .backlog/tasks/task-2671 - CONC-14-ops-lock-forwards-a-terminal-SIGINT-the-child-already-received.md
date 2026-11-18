---
id: TASK-2671
title: 'CONC-14: ops lock forwards a terminal SIGINT the child already received'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2840'
modified_files:
  - crates/cli/src/lock_cmd.rs
priority: medium
ordinal: 1000
dedup_key: 'CONC-14:crates/cli/src/lock_cmd.rs:run_child'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/lock_cmd.rs:590` (`run_child`), `crates/cli/src/lock_cmd.rs:608` (`forward(&child, signo)`), `crates/cli/src/lock_cmd.rs:617` (`forward`)

**What**: `ops lock <name> -- <cmd>` spawns the child in ops's own process group (no `process_group(0)` / `setsid`) and then forwards every caught SIGINT/SIGTERM to it with `libc::kill`. A Ctrl-C at the terminal is delivered by the kernel to the *whole foreground process group*, so the child receives SIGINT once from the tty and a second time from `forward`. A single keypress becomes two signals at the child.

**Why it matters**: Many programs treat a second SIGINT as "abort now, skip cleanup". `ops` itself is one of them: `run_cmd::run_until_signal` restores `SIG_DFL` after the first signal (CONC-14 / TASK-2023), so `ops lock merge -- ops qa` (or any nested `ops <cmd>`) is hard-killed by the forwarded duplicate on one Ctrl-C, bypassing the graceful cancel that tears down in-flight grandchildren. The duplicate also defeats any child that uses "first Ctrl-C = graceful, second = force".

Fix options: put the child in its own process group (`CommandExt::process_group(0)`, then ops is the only one forwarding), or forward only signals that did not come from the terminal (e.g. forward SIGTERM always, SIGINT only when stdin is not a tty / the child is in another pgrp).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single Ctrl-C at the terminal delivers exactly one SIGINT to the locked command
- [ ] #2 SIGTERM sent to ops (not the group) is still forwarded to the child
- [ ] #3 A test covers a nested child that counts the SIGINTs it receives
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
