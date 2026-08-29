---
id: TASK-2037
title: >-
  TEST-15: the ops-run-before-commit has_staged_files suite is flaky — 5 of 5
  tests time out under load and the crate takes 30s even when green
status: Triage
assignee: []
created_date: '2026-08-29 00:05'
updated_date: '2026-08-29 00:44'
labels:
  - code-review-rust
  - testing
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs`

**What**: The `tests::has_staged_files_*` group in `ops-run-before-commit` fails
intermittently. Observed on `code-review/run-20260828-part3` during the wave149
integration verify, on three consecutive standalone runs of
`cargo test -p ops-run-before-commit --lib`:

```
run 1 (inside cargo test --workspace): FAILED. 25 passed; 5 failed   (30.02s)
  tests::has_staged_files_errors_outside_git_repo
  tests::has_staged_files_false_when_index_empty
  tests::has_staged_files_true_when_file_staged
  tests::has_staged_files_true_when_only_a_deletion_is_staged
  tests::has_staged_files_true_when_only_a_type_change_is_staged
run 2 (standalone):                    ok.     30 passed; 0 failed   (30.05s)
run 3 (standalone):                    FAILED. 29 passed; 1 failed   (30.05s)
```

The wall-clock is the tell: every run lands on 30.0s almost exactly, which is a
timeout being consumed rather than work being done. The bounded-wait git probe
these tests drive has a 1500 ms budget (see the TASK-1913 finding, now closed),
and the whole group appears to race it — the failure rate tracks machine load,
which is why the full-workspace run failed all five and the idle standalone run
failed none.

**Not caused by wave149** (TASK-1983): that wave's diff against the landing
branch is confined to `crates/core/src/config/**`, `crates/core/src/text.rs`,
`Cargo.toml`, `README.md`, and `docs/components.md` — it touches no file in
`extensions/`, and the failure reproduces with the wave's own crate untouched.
The crate was reworked on this run branch by the concurrent hook-common waves
(`c96f683 fix(hook-common): count every staged change kind, not just ACMR`,
`aff0b66 fix(run-before-commit): install a POSIX hook that finds ops and arms
the preflight`, `4c22c89 test(hook-common): add a CwdGuard ...`), which is the
likely origin.

Note this is invisible to `ops verify`, which runs fmt / clippy / build / file
hygiene and no tests at all — so CI green on `ops verify` says nothing about it.

**Why it matters**: TEST-15 / flakiness. A test group that fails five-of-five
under load and zero-of-five idle makes every future `cargo test --workspace` run
untrustworthy: the next real regression in any crate gets attributed to "the
flaky hook tests" and waved through. It also costs a fixed 30 s on every
workspace test run even when it passes.

**Origin**: discovered during TASK-1983 (wave149) at the integration-verify step.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The has_staged_files tests pass deterministically under a loaded cargo test --workspace run, repeated at least 10 times
- [ ] #2 The root cause is identified as either a too-tight probe timeout, a shared-cwd race between the tests, or a real hang in the bounded-wait probe — and named in the fix
- [ ] #3 The crate's test wall-clock no longer sits at the timeout budget on a passing run
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Orchestrator follow-up (2026-08-29, end of code-review run 2026-08-28): did NOT reproduce on the quiesced part3 integration branch. `cargo nextest run -p ops-run-before-commit --all-features` green 3/3 standalone; `cargo nextest run --workspace --all-features` green 2871/2871 with has_staged_files passing. The original observation was made while four wave runners were building concurrently, and every failure was pinned at exactly 30.0s -- the env timeout -- which is consistent with CPU starvation under load rather than a code defect. Keeping the task open but the severity is likely overstated: the real question is whether the suite's 30s timeout is too tight to survive a loaded machine, not whether has_staged_files is broken.
<!-- SECTION:NOTES:END -->
