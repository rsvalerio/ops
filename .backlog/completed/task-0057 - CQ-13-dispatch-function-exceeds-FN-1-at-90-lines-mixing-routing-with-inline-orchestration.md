---
id: TASK-0057
title: >-
  CQ-13: dispatch function exceeds FN-1 at 90 lines mixing routing with inline
  orchestration
status: Done
assignee: []
created_date: '2026-04-14 20:48'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - FN-1
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/cli/src/main.rs:113-202 — dispatch is 90 lines with ~16 match arms. Several arms (About, Dashboard, Deps) inline config loading and registry setup rather than delegating to handler functions. The About arm reaches 4 nesting levels via a nested match. Refactoring: extract run_about_dispatch(refresh, action), run_dashboard_dispatch(skip_coverage, refresh) handlers to bring dispatch under 50 lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 dispatch function ≤50 lines with each arm as a single handler call
- [ ] #2 No nested match expressions inside dispatch
<!-- AC:END -->
