---
id: TASK-2637
title: 'Composite steps: declare ordering with after = [...] instead of using exclusive as a stage barrier'
status: To Do
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-10-31 23:55'
labels:
  - feature
  - config
  - scheduling
dependencies: []
parent_task_id: 'TASK-2644'
modified_files: []
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In a parallel plan the only ordering tool is exclusive, which makes a step run alone and splits stages around it. Repos misuse it as a barrier.

dbsec marks fuzz-lock and lint-sync (two fast read-only scripts) exclusive purely so that, in the parallel verify plan, next starts only after build has finished and reuses its artifacts instead of racing it for the cargo lock. Cost: two single-step stages (ops explain verify: stage 2 fuzz-lock, stage 3 lint-sync), the two scripts never overlapping anything, and a comment explaining that exclusive is load-bearing for an unrelated step.

Proposal: a step-level after = ["build"] (or depends_on) honoured inside a parallel plan: the step starts when the named steps in the same plan have finished, and is unconstrained if they are not in the plan. exclusive keeps meaning 'nothing else at the same time'. Cycles and unknown names are load errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A step with after = ["build"] starts only after build completes in a parallel plan
- [ ] #2 Under fail_fast, a failed dependency skips its dependents
- [ ] #3 ops explain shows the dependency edges
- [ ] #4 Cycles and unknown names are load errors naming the commands
<!-- AC:END -->
