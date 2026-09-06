---
id: TASK-2051
title: >-
  TEST-16: the signal-shutdown tests in run_cmd depend on a process-per-test
  harness and would kill a shared cargo test run
status: Done
assignee:
  - TASK-2062
created_date: '2026-08-29 12:54'
updated_date: '2026-08-29 17:43'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - crates/cli/src/run_cmd/tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs` (`signal_shutdown_tests`)

**What**: two tests in this module raise a real `SIGTERM` at their own pid, and one of them (`shutdown_path_restores_default_signal_dispositions`, added by TASK-2023) asserts that `run_until_signal` has reset `SIGINT`/`SIGTERM` to `SIG_DFL` afterwards. That reset is process-global and cannot be undone in-process: tokio registers each OS signal handler behind a `Once`, so a later `signal(SignalKind::terminate())` in the same process will not reinstall it.

Under `cargo nextest` — the project's declared harness — each test runs in its own process and this is harmless. Under a plain in-process `cargo test -p ops`, whichever of the two tests runs second finds the disposition already at `SIG_DFL`, and its `libc::kill(getpid(), SIGTERM)` terminates the whole test binary instead of being caught. `#[serial_test::serial]` prevents overlap but not ordering, so the failure would be intermittent and would look like a harness crash rather than a test failure.

**Why it matters**: the tests are correct under the harness the project uses, but they encode an undeclared dependency on it, and the failure mode when that dependency is not met is a killed test process with no useful diagnostic. Options: gate the pair behind a `#[cfg]`/feature that only nextest sets, run the signal half in a spawned child process (`assert_cmd` on the `ops` binary already exists in this crate), or document the requirement in `AGENTS.md` alongside the nextest command so nobody reaches for `cargo test` on this crate.

**Origin**: discovered during TASK-2043 while fixing TASK-2023.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 the signal-shutdown tests either no longer mutate process-global signal dispositions in a way that affects sibling tests, or the process-per-test requirement is enforced/documented so a plain cargo test run cannot silently kill the harness
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in code-review wave TASK-2062. The two signalling tests in run_cmd::tests::signal_shutdown_tests were collapsed into one, sigterm_interrupts_the_plan_and_restores_default_dispositions, which asserts both TASK-1932 AC #5 (SIGTERM cuts the plan short and reports SIGTERM/143) and CONC-14 / TASK-2023 (SIGINT and SIGTERM are back on SIG_DFL afterwards). Exactly one test in the process now raises a signal, and it leaves the dispositions exactly where the process started, so no sibling test is affected and the undeclared process-per-test dependency is gone rather than merely documented. Verified by running the module under a plain in-process cargo test -p ops --all-features signal_shutdown_tests: 3 passed, no killed harness. The merged test doc comment records why the two assertions must share one test (tokio registers each OS handler behind a Once, so the SIG_DFL reset is not undoable in-process).
<!-- SECTION:NOTES:END -->
