---
id: TASK-0741
title: code-review-plan-wave57
status: In Progress
assignee:
  - code-review-wave
created_date: '2026-04-30 06:06'
updated_date: '2026-04-30 19:42'
labels:
  - code-review-wave
dependencies:
  - TASK-0655
  - TASK-0658
  - TASK-0678
  - TASK-0679
  - TASK-0700
  - TASK-0706
  - TASK-0709
  - TASK-0712
  - TASK-0714
  - TASK-0722
  - TASK-0723
  - TASK-0732
  - TASK-0733
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Performance: allocations, redundant clones, O(n^2) hotpaths
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wave-57 outcome (2026-04-30): 12 of 13 member tasks Done. TASK-0732 (emit_output_events per-line allocations) re-triaged to To Do — fully solving the AC requires moving RunnerEvent::StepOutput::line off String onto a Bytes-style owned-buffer slice, which touches ~40 call sites and the public RunnerEvent JSON shape. That belongs in a dedicated event-API wave, not in this performance-focused one. Wave parent stays In Progress until 0732 is re-triaged. Pre-existing failure in ops-deps `has_issues_ban_warning_not_actionable` (unrelated to wave 57) is reproducible on stashed main; not a regression.
<!-- SECTION:NOTES:END -->
