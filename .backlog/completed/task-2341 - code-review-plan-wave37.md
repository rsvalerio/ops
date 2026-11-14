---
id: TASK-2341
title: 'code-review-plan-wave37'
status: Done
assignee: []
created_date: '2026-09-28 16:38'
updated_date: '2026-09-28 16:56'
labels:
  - code-review-wave
dependencies:
  - TASK-2337
modified_files:
  - .github/workflows/ci.yml
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave37
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rationale: move the lint-actions allow-list from ci.yml --allow into .ops.toml [lint_actions]. Blocker cleared: installed ops 0.75.0 contains [lint_actions] (verified: it accepts the section).
Overlaps: TASK-2334/wave35 (.github/workflows/ci.yml)

Branch: code-review/TASK-2341

<!-- SECTION:NOTES:END -->
