---
id: TASK-1310
title: >-
  TEST-1: cli_run_before_commit_with_malformed_toml_fails asserts only non-zero
  exit
status: To Do
assignee:
  - TASK-1387
created_date: '2026-05-11 19:58'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/tests/integration.rs:481-495`

**What**: Unlike its sibling `cli_run_with_malformed_toml` at integration.rs:457 (which was hardened under TASK-0954 to require the substring `"failed to parse config file: .ops.toml"`), this test only chains `.failure()`. Any unrelated non-zero exit — panic, missing binary, permission error — passes.

**Why it matters**: Inconsistent with the explicit TEST-11 hardening pattern applied to the adjacent test. Leaves a regression gap on the run-before-commit malformed-config path (TASK-0068).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add .stderr(predicate::str::contains("failed to parse config file: .ops.toml")) mirroring cli_run_with_malformed_toml
- [ ] #2 Comment references TEST-11/TASK-0954 alignment so future readers see the intent
<!-- AC:END -->
