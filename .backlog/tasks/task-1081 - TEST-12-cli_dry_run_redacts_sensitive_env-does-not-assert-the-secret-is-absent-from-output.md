---
id: TASK-1081
title: >-
  TEST-12: cli_dry_run_redacts_sensitive_env does not assert the secret is
  absent from output
status: Done
assignee: []
created_date: '2026-05-07 21:20'
updated_date: '2026-05-08 06:17'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:382-403`

**What**: The test asserts `***REDACTED***` is present and `"visible"` is present, but never `not(predicate::str::contains("super_secret_value"))`. A regression that printed the redacted line AND leaked the raw value (e.g. duplicated table render) would silently pass.

**Why it matters**: This is the canonical secret-redaction test. Its only job is to prove the secret never reaches stdout. Without a negative assertion, the test cannot catch the regression it exists to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add .stdout(predicate::str::contains("super_secret_value").not())
- [ ] #2 Also assert the secret is absent from stderr
- [ ] #3 Extend with a second variant covering composite-command dry-run
<!-- AC:END -->
