---
id: TASK-0042
title: 'CQ-11: theme/src/lib.rs is thick — 398 lines of implementation in entry point'
status: Done
assignee: []
created_date: '2026-04-14 20:14'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - ARCH-8
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/theme/src/lib.rs contains the full StepLineTheme trait definition, ConfigurableTheme implementation, all rendering helpers (render_step_line, render_plan_header, render_separator, render_error_block), and format_duration — all in a single 398-line lib.rs file. Per ARCH-8, lib.rs should be a thin entry point with module declarations and re-exports. Implementation belongs in dedicated submodules (e.g. theme.rs for the trait, render.rs for rendering helpers, duration.rs or util.rs for format_duration).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 lib.rs reduced to module declarations and re-exports — under 30 lines
- [ ] #2 Implementation split into at least 2 submodules by responsibility
<!-- AC:END -->
