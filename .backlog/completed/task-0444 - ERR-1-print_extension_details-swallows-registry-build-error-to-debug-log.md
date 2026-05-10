---
id: TASK-0444
title: 'ERR-1: print_extension_details swallows registry build error to debug log'
status: Done
assignee:
  - TASK-0536
created_date: '2026-04-28 05:43'
updated_date: '2026-04-28 16:12'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:233-236`

**What**: When `build_data_registry` fails inside `print_extension_details`, the error is logged at `tracing::debug!` and the schema section is silently dropped from the output. Users running `ops extension show` get a partial display with no signal that the schema rendering failed.

**Why it matters**: Hides extension wiring problems behind a debug-level log the user almost certainly does not have enabled; the rest of `extension show` succeeds, masking the failure entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either propagate the error (extension show already returns anyhow::Result) or surface a visible note on stdout/stderr indicating schema unavailability with cause
- [x] #2 Regression test constructs a config triggering registry-build failure and asserts the error reaches the user
<!-- AC:END -->
