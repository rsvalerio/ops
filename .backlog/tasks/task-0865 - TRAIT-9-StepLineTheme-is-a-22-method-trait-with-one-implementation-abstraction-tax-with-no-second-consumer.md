---
id: TASK-0865
title: >-
  TRAIT-9: StepLineTheme is a 22-method trait with one implementation;
  abstraction tax with no second consumer
status: Triage
assignee: []
created_date: '2026-05-02 09:21'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:167-443`

**What**: The trait carries 22 methods, most defaulted, and is exclusively implemented by ConfigurableTheme. The doc on lines 147-156 already records the deferred ARCH-2 decision but explicitly says "Revisit when a second non-configurable theme is added." That trigger has not occurred and the trait keeps growing (boxed-layout methods were added).

**Why it matters**: TRAIT-9 - start concrete until a second impl justifies abstraction. Each new method on the trait is paid for in default-impl maintenance and vtable dispatch on the render path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Re-evaluate per the documented criterion (second non-configurable theme exists?). If no, plan to fold into ConfigurableTheme
- [ ] #2 If yes, split into per-concern traits (StepRender, BoxLayout, ErrorBlock)
- [ ] #3 Document the decision outcome in the trait header or close the deferred review
<!-- AC:END -->
