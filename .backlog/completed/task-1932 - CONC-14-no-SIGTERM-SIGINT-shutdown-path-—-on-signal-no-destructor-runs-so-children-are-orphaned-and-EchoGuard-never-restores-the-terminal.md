---
id: TASK-1932
title: >-
  CONC-14: no SIGTERM/SIGINT shutdown path — on signal no destructor runs, so
  children are orphaned and EchoGuard never restores the terminal
status: Done
assignee:
  - TASK-1986
created_date: '2026-08-27 15:46'
updated_date: '2026-08-28 19:25'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/runner/src/terminal.rs
  - crates/runner/src/command/exec.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/terminal.rs:102-120` (`impl Drop for EchoGuard`), `crates/runner/src/command/build.rs:626` (`kill_on_drop(true)`), consumed at `crates/cli/src/run_cmd.rs:296`

**What**: the runner has no signal handling at all. `grep -rn 'ctrl_c|SignalKind|signal_hook|SIGINT' --include=*.rs crates/` finds only `SIGINT_EXIT`, an exit-code constant (`crates/cli/src/main.rs:91`) used to report a *prompt* cancellation — not a handler. `tokio::signal` is not used anywhere, and `ops-runner` does not depend on it.

Both of the runner's cleanup mechanisms are `Drop`-based, and `Drop` does not run when the process is terminated by a signal:

1. **`EchoGuard`** clears `ECHO` on the stderr tty at the start of a run and restores the saved `termios` in `Drop` (`terminal.rs:102-120`). Killed by a signal, the restore never happens and the user is returned to a shell where their typing is invisible. The guard's own doc sells the RAII shape ("restores echo on drop") without noting that the one interruption users actually perform — Ctrl-C on a long build — bypasses it.
2. **`kill_on_drop(true)`** is the only thing that kills a running child. On `SIGTERM` (a CI job cancellation, a `kill`, a container stop) the signal goes to the `ops` pid alone, `Drop` never runs, and every in-flight child survives the runner that spawned it. `SIGINT` from a terminal reaches the foreground process group so children usually die too, but that is the tty's doing, not the runner's, and it does not hold for `SIGTERM`, for a non-tty invocation, or for children that ignore/handle `SIGINT` themselves.

The two concerns share one fix: an explicit shutdown path that owns both. `tokio::signal::unix::signal(SignalKind::terminate())` alongside `signal::ctrl_c()`, raced against the plan future, which on trip (a) trips the existing `AbortSignal` and cancels the `JoinSet` so the runner's own kill path runs, (b) restores the terminal, and (c) exits with the conventional `128 + signo`. The `AbortSignal` (`command/abort.rs`) and the `fail_fast` cancellation machinery already exist and are exactly the right target — there is simply nothing wired to a signal to trip them.

This compounds TASK-1919 (`kill_on_drop` reaches only the direct child): fixing process-group ownership without a signal path still leaves the tree alive on `SIGTERM`, because nothing runs to send the group signal.

**Why it matters**: Ctrl-C on a running build and a cancelled CI job are the two most common ways an `ops` run ends early. Today the first can leave the terminal unusable and the second reliably leaks the entire child process tree onto the runner host.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 the runner (or the entry point that drives it) installs a shutdown path covering both SignalKind::terminate() and ctrl_c(), not ctrl_c alone
- [x] #2 on signal the existing AbortSignal is tripped and in-flight children are killed through the runner's own cancellation path rather than relying on Drop
- [x] #3 terminal state is restored on the signal path so a Ctrl-C'd run does not leave the user's tty with ECHO cleared
- [x] #4 EchoGuard's doc comment states which termination paths run its Drop and which do not
- [x] #5 a test or documented manual procedure covers SIGTERM: children are gone and the process exits with the conventional 128+signo code
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
CLI now owns an explicit shutdown path: run_cmd::run_until_signal races the plan future against BOTH SignalKind::terminate() and SignalKind::interrupt() (the signal ctrl_c() listens for on unix — used directly so the signal number is available for the exit code), wrapping the display path and the raw path. On signal the losing select arm DROPS the plan future, which is the runner own cancellation path: the JoinSet is dropped/aborted and each spawn_capped ChildGroup guard signals the child process group (CONC-9 / TASK-1919), so children die instead of being orphaned. AC #2 note: the internal AbortSignal is not tripped from the CLI — it is private to run_plan_parallel and exposing it would be new public API for no extra effect, since dropping the JoinSet cancels not-yet-started tasks and kills in-flight ones through the same path the signal is meant to trigger. The EchoGuard is dropped on the signal path before returning, so the terminal ECHO bit is restored, and the run exits 128+signo via the existing ExitCodeOverride channel (130 SIGINT / 143 SIGTERM). Handler registration failure degrades to the previous plain-await with a warn. EchoGuard docs gained a table of which termination paths run its Drop. Tests: signal_exit_codes_follow_the_128_plus_signo_convention, plan_completes_normally_when_no_signal_arrives, and sigterm_interrupts_the_plan_and_reports_sigterm — which raises a REAL SIGTERM at its own pid against a never-completing plan future, so the signal arm is the only way the test can return. Added the tokio "signal" feature workspace-wide and libc as an ops-cli dev-dependency for that test.
<!-- SECTION:NOTES:END -->
