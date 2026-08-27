---
id: TASK-1753
title: >-
  TEST-26: run_command_cli_full_lifecycle is permanently #[ignore]d for a reason
  its un-ignored siblings contradict
status: Triage
assignee: []
created_date: '2026-08-27 11:15'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/cli/src/run_cmd/tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs:36-58`

**What**: `run_command_cli_full_lifecycle` is the only test in the crate that asserts the runner's **event stream** — that a run emits `RunnerEvent::PlanStarted`, `RunnerEvent::StepFinished`, and `RunnerEvent::RunFinished { success: true, .. }`. It is `#[ignore]`d:

```rust
#[tokio::test(flavor = "multi_thread")]
#[ignore = "spawns real subprocesses; run with --ignored. Validates full CLI lifecycle."]
```

with a doc block justifying the ignore as: spawns real subprocesses, writes to stderr, requires `echo` to be available.

None of those three reasons distinguishes it from tests that run by default in the same file and the same workspace:

- `run_command_returns_success_for_valid_command` (`run_cmd/tests.rs:134`) spawns `echo` through the full runner, not ignored.
- `run_command_returns_failure_for_failing_command` (line 158) spawns `false` / `cmd /C exit 1`, not ignored.
- `cli_run_echo_reports_resolved_command_and_timing_in_stderr` (`tests/integration.rs:250`) spawns the whole `ops` binary running `echo` and asserts on its stderr, not ignored.

So the stated justification is stale: the project already accepts real-subprocess, stderr-writing, `echo`-dependent tests in the default run. The consequence is that the *only* coverage of `RunnerEvent` emission from the CLI side never executes in `ops next` / `ops qa-next`, and a regression that stopped emitting `PlanStarted` or flipped `RunFinished { success }` would be caught by nothing. The "Re-enable criteria" in the doc block ("run with `--ignored`", "or mock subprocess execution") has no owner and no tracking task.

Note also that TASK-1664 ("CI Test job runs only ignored tests") is closed, so the ignored set is not compensated for by a separate CI job.

**Why it matters**: TEST-26 — an ignored test is silent coverage loss. This one is worse than an unexplained ignore because the explanation reads as sound until it is checked against the tests running beside it, which makes it unlikely anyone will revisit the decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_command_cli_full_lifecycle either runs by default (the #[ignore] and its stale justification removed) or its ignore reason is rewritten to a fact that actually distinguishes it from run_command_returns_success_for_valid_command
- [ ] #2 If it stays ignored, the reason names a concrete blocker and a tracking task id, and the RunnerEvent emission assertions are relocated into a test that does run by default
- [ ] #3 Either way, PlanStarted / StepFinished / RunFinished{success} emission from the CLI run path is asserted by at least one test that executes in the default 'ops next' run
<!-- AC:END -->
