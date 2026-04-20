---
id: TASK-0078
title: 'TEST-26: #[ignore]d full-lifecycle test hides regressions by default'
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 15:12'
labels:
  - rust-codereview
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:345`

**What**: run_command_cli_full_lifecycle is #[ignore]d with a clear reason but is the only test exercising the full PlanStarted -> RunFinished path end-to-end.

**Why it matters**: The lifecycle contract is tested only when someone remembers --ignored; regressions could land unnoticed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Run this test in CI via a dedicated --ignored pass
- [ ] #2 Or refactor to mock subprocess so the test can run by default
<!-- AC:END -->
