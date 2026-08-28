---
id: TASK-1918
title: >-
  ASYNC-6: spawn_capped captures stdout/stderr but leaves the child's stdin
  inherited, so an input-prompting child hangs the runner forever
status: To Do
assignee:
  - TASK-1986
created_date: '2026-08-27 15:43'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - async
dependencies: []
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/build.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:111-115` (`spawn_capped`), reached from `exec_command` (`exec.rs:429`)

**What**: `spawn_capped` configures only two of the three stdio slots:

```rust
cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
let mut child = cmd.spawn()?;
```

`tokio::process::Command` defaults stdin to `Stdio::inherit()`, so every captured child inherits the runner's fd 0. Nothing in `build_command_with` (`build.rs:600-628`) sets stdin either — only the deliberately-inherited raw path (`exec_command_raw`, `exec.rs:509-511`) sets all three explicitly.

Consequences:

1. A child that reads stdin (`git` asking for a passphrase / credential, `sudo`, `ssh`, `npm login`, an interactive migration tool, a REPL invoked by mistake) blocks on `read(0)`. Its prompt was written to the *captured* pipe, so the user sees a spinner with no prompt and no indication that input is wanted.
2. `spec.timeout()` is `Option<Duration>` from `timeout_secs: Option<u64>` (`crates/core/src/config/commands.rs:131`) and is `None` unless the `.ops.toml` author opted in, so `run_with_timeout` degrades to a bare `future.await` (`exec.rs:60-70`) and the hang is unbounded. In CI this is a job that runs until the runner's wall-clock limit.
3. Under `run_plan_parallel` up to `OPS_MAX_PARALLEL` (default 32) children share the same terminal fd 0 simultaneously, so whichever ones do read stdin race each other for the user's keystrokes while the display owns the screen.

The captured path has no reason to hand the child a readable stdin: its whole contract is "no interaction, we own the output". `Stdio::null()` turns the hang into an immediate EOF and the child fails fast with its own diagnostic, which then lands in the captured stderr the display already renders.

**Why it matters**: an interactive child is not exotic in a build-tool step list (`git push` over HTTPS, `docker login`, a `cargo publish`). Today that step wedges `ops` with no output, no timeout, and no way for the user to tell what it is waiting for; on the parallel path it also wedges every sibling behind the display pump. This is a reliability bug in the engine's core spawn path, not a UI nicety.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 spawn_capped (or build_command_with, for the captured path only) sets stdin to Stdio::null() so a captured child cannot block on terminal input
- [ ] #2 exec_command_raw keeps Stdio::inherit() for stdin — raw mode is the documented interactive path and must not regress
- [ ] #3 a regression test spawns a child that reads stdin (e.g. sh -c 'read x') through the captured path and asserts the step terminates without an external timeout
- [ ] #4 the module docs on exec.rs state that captured steps get no stdin and that interactive commands must use --raw
<!-- AC:END -->
