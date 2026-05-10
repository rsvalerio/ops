---
id: TASK-0461
title: 'ARCH-2: StepLineTheme trait exposes 22 methods with one production impl'
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 16:54'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:141-417`

**What**: StepLineTheme exposes 22 methods spanning padding, icons, colors, header/summary, progress, step rendering, boxed layout, error detail. The trait doc notes only status_icon is required and defaults read off ConfigurableTheme. There is exactly one production impl.

**Why it matters**: Comment at step_line_theme.rs:130-140 already calls out the smell. With one impl, the trait is overhead — every call site goes through a vtable. With API stabilised, worth re-evaluating: split into smaller traits, or replace with a concrete struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decision recorded (or refactor executed): split into 3 cohesive traits, collapse to a concrete struct + free fns, or keep as-is with a documented criterion (revisit when N-th impl arrives)
- [x] #2 If kept as a single trait, forward-compat strategy is documented (defaults shield downstream, but adding a required method still breaks)
<!-- AC:END -->
