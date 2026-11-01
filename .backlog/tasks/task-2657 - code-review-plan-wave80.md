---
id: TASK-2657
title: 'code-review-plan-wave80'
status: To Do
assignee: []
created_date: '2026-11-01 10:31'
updated_date: '2026-11-01 11:04'
labels:
  - code-review-wave
dependencies:
  - TASK-2647
  - TASK-2648
modified_files:
  - .ops.toml
  - rustfmt.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave80
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Rationale: the ops repo must pass its own doc-ids and foundation checks so both can join a local gate; both follow-ups from TASK-2645. Scope is a lower bound: .ops.toml is what the tasks declare, but clearing 989 doc-comment ids (2647) touches many sources across the workspace, and adopting rustfmt.toml (2648) may reformat the tree, unless a waiver is recorded. Expect wide conflicts: land after the other waves.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none declared. The wave overlap tool sees only .ops.toml and rustfmt.toml, but clearing the doc-comment ids (2647) and any rustfmt adoption (2648) touch sources widely, so this will really overlap TASK-2655 and TASK-2656. Merge it last despite the suggested order.

Prerequisite: doc-ids and foundation exist only on main after #120, in no published release (v0.90.0 predates them and its release run was cancelled). Before wiring either into verify or qa, cut a release containing #120 and bump mise.toml, or CI fails with unknown command.

<!-- SECTION:NOTES:END -->
