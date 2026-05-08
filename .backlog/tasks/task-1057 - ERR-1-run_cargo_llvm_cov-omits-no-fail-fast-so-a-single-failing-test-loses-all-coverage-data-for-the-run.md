---
id: TASK-1057
title: >-
  ERR-1: run_cargo_llvm_cov omits --no-fail-fast, so a single failing test loses
  all coverage data for the run
status: Done
assignee: []
created_date: '2026-05-07 21:04'
updated_date: '2026-05-08 06:52'
labels:
  - code-review
  - extensions-rust
  - test-coverage
  - ERR-1
  - PATTERN-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/test-coverage/src/lib.rs:94-107 invokes `cargo llvm-cov --workspace --no-cfg-coverage --tests --json` and check_llvm_cov_output then bails on any non-zero status. Without --no-fail-fast (passed through to cargo test), the first failing test aborts the suite; cargo llvm-cov returns non-zero and the JSON output is either absent or partial. CoverageProvider/load_coverage then fails the entire collection, so the coverage_files / coverage_summary tables are never populated even though the passing tests' coverage data is recoverable.

Operationally: a single broken test on a feature branch erases the coverage signal in about / deps reports until the test is fixed, and CI loses observability into coverage trends across an in-flight test fix.

Fix: add --no-fail-fast to the cargo llvm-cov args. cargo llvm-cov's exit status will then reflect the wrap-up of all tests and coverage JSON is written even when some tests fail. Optionally, treat a non-zero exit with valid JSON on stdout as a soft failure (warn + continue) so the user still sees per-file coverage for the passing slice of the workspace.

Sister pattern: matches metadata provider's choice to surface partial-but-useful data on subprocess hiccups.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_cargo_llvm_cov passes --no-fail-fast
- [x] #2 Coverage JSON is produced even when one or more tests fail
- [x] #3 check_llvm_cov_output (or sibling) tolerates non-zero exit when stdout contains a parseable llvm-cov JSON document, logging the test failures at warn rather than dropping all coverage data
- [x] #4 Regression test pins the new arg list
<!-- AC:END -->
