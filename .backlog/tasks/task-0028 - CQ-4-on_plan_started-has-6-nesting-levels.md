---
id: TASK-0028
title: 'CQ-4: on_plan_started has 6 nesting levels'
status: Done
assignee: []
created_date: '2026-04-14 19:14'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - FN-2
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/runner/src/display.rs:272-305 (34 lines) — on_plan_started builds step entries with a triple-nested map closure (map → and_then → unwrap_or) inside a collect, reaching 6 nesting levels. The dense closure at lines 274-288 could be extracted to a build_step_entries() helper to flatten nesting and improve readability. Violates FN-2 (≤4 nesting levels). Affected crate: ops-runner.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract step entry construction to a named helper. Nesting ≤4 levels.
<!-- AC:END -->
