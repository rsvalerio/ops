---
id: TASK-1938
title: >-
  TEST-6: collect_coverage has no seam for the cargo runner, so its soft-fail
  demotion, temp-file wiring, and non-UTF-8 path arm are all untested
status: Triage
assignee: []
created_date: '2026-08-27 15:47'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:337-393`

**What**: `collect_coverage` is the crate's collection entry point — everything `CoverageIngestor::collect` produces flows through it. It calls `run_cargo_llvm_cov` directly, with no injected runner, so nothing in the suite can drive it past the subprocess spawn. The result is that five distinct behaviours have zero coverage:

1. The success path — cargo exits 0, the report file is read, parsed, and flattened. No test reaches it.
2. The soft-fail demotion — cargo exits non-zero, the report parses with a non-empty `data[]` carrying `files`, and the function warns and returns partial coverage instead of failing. The predicate is tested (against a copy of itself — see the DUP-1 finding on the same file), but the demotion it drives is not.
3. The hard-fail fall-through — the predicate rejects, `check_llvm_cov_output(&output)?` bails. `check_llvm_cov_output` is tested directly with synthetic `Output` values, but that the fall-through actually reaches it is not.
4. The non-UTF-8 temp path arm — `report.path().to_str().context("llvm-cov temp report path is not valid UTF-8")` returns an error no test constructs.
5. The report-read arms — `std::fs::read(report.path()).ok()` on the soft-fail side and `.context("reading llvm-cov JSON report")?` on the success side.

The only test that touches the function at all is `coverage_ingestor::tests::coverage_collect_fails_with_nonexistent_directory` (`ingestor.rs:44-53`), which points the working directory at a path that does not exist so the spawn fails, and asserts only `result.is_err()`. That covers the spawn-failure arm and nothing after it.

The reason a test double is needed rather than a real invocation: a real `collect_coverage` run executes the whole workspace test suite under instrumentation with a 15-minute timeout (`CARGO_LLVM_COV_TIMEOUT`), which is not a unit test. The fix is a seam — take the runner as a parameter (a `fn(&Path, &str) -> Result<Output, RunError>` or a small trait), keep `collect_coverage` as the thin wrapper that passes `run_cargo_llvm_cov`, and test the inner function with synthetic `Output` values plus a caller-written report file, the same way `check_llvm_cov_output`'s tests already build `std::process::Output` from `ExitStatusExt::from_raw`.

Sibling precedent: TASK-1787 filed the same "no seam to inject a subprocess result" shape against `ops-cargo-update`.

**Why it matters**: the soft-fail branch is where a genuine cargo failure gets demoted to a warning and the run continues with partial data. TASK-1057, TASK-1557, and TASK-1597 were all bugs in exactly this branch, and after all three fixes there is still no test that executes it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 collect_coverage is split so the cargo runner is injectable, with the public entry point remaining a thin wrapper that passes run_cargo_llvm_cov
- [ ] #2 Tests drive the injectable form with a synthetic zero-exit Output plus a written report file and assert the flattened rows come back
- [ ] #3 Tests drive the soft-fail demotion (non-zero exit, report with non-empty data[] carrying files) and assert partial rows are returned rather than an error
- [ ] #4 Tests drive the hard-fail fall-through (non-zero exit, report that fails the predicate) and assert the surfaced error names the cargo exit, not a schema-shape parse failure
- [ ] #5 The report-read failure arm is exercised: a non-zero exit whose report file is absent or unreadable falls through to the cargo error rather than being reported as a JSON problem
<!-- AC:END -->
