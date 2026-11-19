---
id: TASK-2866
title: 'runner: raw-mode timeout should tear down the step''s process tree via a foreground process-group hand-off'
status: To Do
assignee: []
created_date: '2026-11-18 21:22'
updated_date: '2026-11-19 08:07'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2880'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Not implemented in wave28 (TASK-2880): no part of it could be landed safely and tested in this wave. A foreground hand-off (child as its own group leader, tcsetpgrp to the child with SIGTTOU blocked, tcsetpgrp back on every exit path through a Drop guard) also needs these, which today's shared-group design gets free from the user's shell:
- Job control: Ctrl-Z sends SIGTSTP only to the child's group, so the child stops and the runner keeps waiting in child.wait(). tokio's wait cannot report stops (no WUNTRACED), so the terminal hangs on a stopped foreground group. Fixing that means the runner implements job control itself: waitpid(WUNTRACED), take the tty back, stop itself, then hand the tty back and SIGCONT the group on resume.
- Background runs: ops started with & is not the tty's foreground group, and a tcsetpgrp with SIGTTOU blocked would take the terminal from the shell. The hand-off has to be gated on tcgetpgrp(tty) == getpgrp(), with the no-controlling-tty case handled separately.
- Ctrl-C relay: SIGINT reaches only the child. The runner must re-raise SIGINT on itself after a child killed by SIGINT, so run_until_signal still cancels the run. A child that catches SIGINT and keeps running (a REPL) would no longer cancel the run, which changes today's behaviour.
- Tests: AC#2 needs a pty-driven test (a controlling terminal, the foreground group, Ctrl-C injection), and the crate has no pty harness yet.
The documented trade-off in exec_command_raw and docs/configuration.md stays accurate. This needs its own wave with a pty test harness.
<!-- SECTION:NOTES:END -->
