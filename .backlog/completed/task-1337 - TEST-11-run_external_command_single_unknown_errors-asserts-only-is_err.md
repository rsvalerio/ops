---
id: TASK-1337
title: 'TEST-11: run_external_command_single_unknown_errors asserts only is_err()'
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-12 16:27'
updated_date: '2026-05-17 09:31'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs:322-331`

**What**: The single-unknown variant asserts only `result.is_err()`, while the multi-arg empty-path sibling at line 270 already inspects the message (`"missing command"`). A regression where empty-config loading fails before unknown-command resolution still passes silently.

**Why it matters**: Test name encodes a specific failure mode but the assertion does not pin it. The style at `dry_run_returns_error_for_unknown_command:463` (expect_err + format!("{err:#}") substring) is the established pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Assertion checks the error chain mentions the unknown command id (e.g. "nonexistent").
- [x] #2 Uses the expect_err + format!("{err:#}") substring style already present in the file.
<!-- AC:END -->
