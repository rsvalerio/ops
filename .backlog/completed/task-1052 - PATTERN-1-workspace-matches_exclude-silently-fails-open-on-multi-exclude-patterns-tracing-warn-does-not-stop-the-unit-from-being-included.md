---
id: TASK-1052
title: >-
  PATTERN-1: workspace::matches_exclude silently fails-open on multi-* exclude
  patterns; tracing::warn does not stop the unit from being included
status: Done
assignee: []
created_date: '2026-05-07 21:02'
updated_date: '2026-05-07 23:29'
labels:
  - code-review
  - about
  - workspace
  - PATTERN-1
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions/about/src/workspace.rs:111-135 — matches_exclude returns false when the pattern contains more than one *, after emitting tracing::warn. resolve_member_globs calls it via .iter().any(...) so a multi-* exclude evaluates as 'no match' and the candidate stays in the resolved list — failing open. Workspace excludes are typically used to keep internal/private modules out of the published surface; a typo like 'packages/*-internal-*' (which the user thinks is a valid extended-glob) silently leaks the unit into about/cards/identity output rather than failing closed.

Either reject multi-* patterns at config-load time (hard error) so the user is forced to fix it, or treat the unsupported pattern as 'matches everything for safety' (fail closed) so the wrong default is being too restrictive instead of too permissive. The current 'warn and let through' pattern is the worst-of-both: the operator only sees one warn line per run and the offending unit ships in production output until someone notices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 matches_exclude does not silently let through a candidate when an unsupported pattern is encountered
- [ ] #2 Documented behaviour in resolve_member_globs reflects the new fail-mode
<!-- AC:END -->
