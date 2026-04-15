---
id: TASK-0056
title: >-
  CQ-12: render_detail_section exceeds FN-1 at 109 lines with repetitive
  match-arm pattern
status: Done
assignee: []
created_date: '2026-04-14 20:48'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - FN-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/about/src/identity.rs:117-226 — render_detail_section is 109 lines (2x FN-1 threshold). Five match arms repeat query→format→empty-check→wrap pattern. The 'coverage' and 'stats' arms duplicate header-stripping logic (skip_while + section heading). Refactoring: extract format_section(lines, strip_header) helper to deduplicate the repeated pattern across arms.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_detail_section ≤50 lines or each match arm delegated to a named helper
- [ ] #2 Header-stripping logic appears exactly once (DRY)
<!-- AC:END -->
