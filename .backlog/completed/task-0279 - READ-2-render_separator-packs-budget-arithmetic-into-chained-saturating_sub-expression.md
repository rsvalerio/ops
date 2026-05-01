---
id: TASK-0279
title: >-
  READ-2: render_separator packs budget arithmetic into chained saturating_sub
  expression
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:26'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:347`

**What**: template_overhead, line_budget, space_for_sep, sep_count computed without names.

**Why it matters**: Hard to debug "why separator width is N".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract available_width/sep_slots intermediates
- [ ] #2 Add comment explaining the +1 fudge
<!-- AC:END -->
