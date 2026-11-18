---
id: TASK-2866
title: 'runner: raw-mode timeout should tear down the step''s process tree via a foreground process-group hand-off'
status: Triage
assignee: []
created_date: '2026-11-18 21:22'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/process_group.rs
  - docs/configuration.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs` (`exec_command_raw`), `crates/runner/src/command/process_group.rs`

**What**: A raw step that exceeds `timeout_secs` still loses only its direct child to `kill_on_drop`; descendants (`cargo` under `sh -c`, `node` under `npm run`) keep running on the terminal. TASK-2700 recorded this as a deliberate, documented trade-off (doc on `exec_command_raw`, `docs/configuration.md` "Raw commands"), because a raw child must share the tty's foreground process group for Ctrl-C to reach it and the runner. The real fix is to spawn raw children as their own process-group leader, hand them the terminal with `tcsetpgrp` (blocking SIGTTOU while doing so, and taking it back after the child exits), relay Ctrl-C to the runner's cancellation path, and tear the group down through `ChildGroup` on timeout.

**Why it matters**: Orphans keep writing over the next step's output and can hold `.cargo-lock`, blocking the next build.

**Origin**: discovered during TASK-2840 while fixing TASK-2700 (deferred design work).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A raw step that times out leaves no descendants running (unix test with 'sh -c "sleep 30 & wait"')
- [ ] #2 A Ctrl-C at the terminal during a raw step still cancels the run as it does today
- [ ] #3 The documented limitation in exec_command_raw and docs/configuration.md is removed
<!-- AC:END -->
