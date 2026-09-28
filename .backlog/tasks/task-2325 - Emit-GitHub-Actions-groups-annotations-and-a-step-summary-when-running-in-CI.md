---
id: TASK-2325
title: 'Emit GitHub Actions groups, annotations and a step summary when running in CI'
status: To Do
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:07'
labels:
  - ci
  - ops-alignment
dependencies: []
parent_task_id: 'TASK-2332'
modified_files: []
priority: medium
ordinal: 1000
dedup_key: 'ops-align:ops-ci-output'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: ops emits no `::group::`, `::error` annotations, step summary or JUnit; a failing step only shows stderr (forge had to wrap actionlint/check-yaml in `sh -c '... 1>&2'` to see failures).

**Why it matters**: once CI runs `ops verify`/`qa`, failures must be as readable as the per-job cargo output they replace.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Under GITHUB_ACTIONS, each composite step is a collapsible group and a failure is annotated with the command and its output tail
- [ ] #2 A run summary is written to GITHUB_STEP_SUMMARY
<!-- AC:END -->
