---
id: TASK-0278
title: >-
  FN-3: build_horizontal_border carries #[allow(too_many_arguments)] with 6
  params
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:25'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:239`

**What**: title, left_corner, right_corner, columns, left_pad, title_color.

**Why it matters**: Allow-annotated smell — positional params invite bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce BorderSpec struct
- [ ] #2 Drop the allow attribute
<!-- AC:END -->
