---
id: TASK-1372
title: >-
  TEST-1: cli_run_echo_success asserts stderr timing suffix but never observes
  echo output reaching stdout
status: Done
assignee:
  - TASK-1387
created_date: '2026-05-12 21:42'
updated_date: '2026-05-13 08:39'
labels:
  - code-review-rust
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/tests/integration.rs:240-256`

**What**: The test pipes `args = ["integration_test_output"]` through an `echo` command but only asserts `success()` and that stderr contains the timing suffix `" in "`. The integration-test name claims to validate echo success; the actual contract under test is "echo runs and reports timing in stderr", which any program that exits 0 (e.g. `true`) would also satisfy. The unique value being echoed (`integration_test_output`) never appears in any assertion.

**Why it matters**: A regression that wired `ops <cmd>` to a no-op subprocess driver, dropped stdout capture, or swapped the resolved program (e.g. ran a stale config) would still pass this test. The test name reads as a coverage win for the end-to-end run path while exercising only the timing-report side-channel. Aligns with the calibration sweep that promoted similar TEST-11/TEST-25 weak-assert findings under TASK-1299 / TASK-0954.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 test asserts the unique args payload appears in the captured stdout (use Command::output() or .stdout(predicate::str::contains("integration_test_output")))
- [ ] #2 if echo output is unobservable through assert_cmd in this code path, the test is renamed to clarify what it actually exercises (timing line in stderr) or removed
<!-- AC:END -->
