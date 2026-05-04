---
id: TASK-0954
title: >-
  TEST-11: integration error-path tests assert only .failure() without verifying
  error message
status: Triage
assignee: []
created_date: '2026-05-04 21:45'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:117-132, 286-300, 430-448`

**What**: `cli_run_unknown_command_fails`, `cli_run_with_malformed_toml`, and `cli_with_invalid_ops_d_file` only check non-zero exit. They don't verify stderr identifies the actual cause (unknown command vs config parse error vs `.ops.d/` parse error). A regression where the binary fails for an unrelated reason (missing `.ops.toml`, unrelated panic) still passes all three.

**Why it matters**: Three distinct error paths conflated under a generic `.failure()` assertion — TEST-11 requires asserting specific values, not just is_err.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each test asserts a stderr substring identifying its specific failure mode
- [ ] #2 Substring assertions are specific enough that swapping any of the three scenarios fails the others
<!-- AC:END -->
