---
id: TASK-0072
title: 'CL: on_step_failed carries 26-line doc comment about its own complexity'
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - cl
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:423`

**What**: on_step_failed has a 26-line doc explaining its 4-level nesting and promising future extraction of render_error_details_tty/non_tty helpers.

**Why it matters**: The comment signals known cognitive-load debt; extracting the two helpers as described would remove the explanation and reduce nesting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract render_error_details_tty and render_error_details_non_tty per the doc promise
- [ ] #2 Drop the nesting-explanation block once the helpers exist
<!-- AC:END -->
