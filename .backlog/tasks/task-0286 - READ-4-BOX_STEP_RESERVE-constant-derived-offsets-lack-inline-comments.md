---
id: TASK-0286
title: 'READ-4: BOX_STEP_RESERVE constant derived offsets lack inline comments'
status: To Do
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:11`

**What**: Uses `BOX_STEP_RESERVE as usize - 2` and `frame_overhead = 2 * left_pad + BOX_STEP_RESERVE` without inline justification.

**Why it matters**: Readers must reverse-engineer which -2 subtracts the two frame bars.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add named helpers inner_budget/gutter_for_mid
- [ ] #2 Comment each offset
<!-- AC:END -->
