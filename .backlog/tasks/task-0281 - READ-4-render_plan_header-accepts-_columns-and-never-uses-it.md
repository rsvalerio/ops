---
id: TASK-0281
title: 'READ-4: render_plan_header accepts _columns and never uses it'
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
**File**: `crates/theme/src/step_line_theme.rs:172`

**What**: Dead parameter; Tree branch could wrap but does not.

**Why it matters**: Incomplete API contract signals future work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document why columns is accepted
- [ ] #2 Or drop from signature
<!-- AC:END -->
