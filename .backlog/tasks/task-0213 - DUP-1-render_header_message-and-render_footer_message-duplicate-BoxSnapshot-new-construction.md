---
id: TASK-0213
title: >-
  DUP-1: render_header_message and render_footer_message duplicate
  BoxSnapshot::new construction
status: To Do
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:307`

**What**: Both methods compute elapsed and build an identical BoxSnapshot::new(...).with_command_ids(...) pattern.

**Why it matters**: Two copies of the same snapshot-building logic drift independently; a change to tracking success or elapsed must be applied twice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a private fn current_box_snapshot(&self) -> BoxSnapshot helper
- [ ] #2 Call it from both render_header_message and render_footer_message
<!-- AC:END -->
