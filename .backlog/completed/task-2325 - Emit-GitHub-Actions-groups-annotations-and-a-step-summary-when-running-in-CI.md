---
id: TASK-2325
title: 'Emit GitHub Actions groups, annotations and a step summary when running in CI'
status: Done
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:25'
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
- [x] #1 Under GITHUB_ACTIONS, each composite step is a collapsible group and a failure is annotated with the command and its output tail
- [x] #2 A run summary is written to GITHUB_STEP_SUMMARY

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in crates/runner/src/display/github.rs (wave TASK-2332): per-step ::group:: fenced with ::stop-commands::<random token>, ::error annotation with command/message/10-line output tail, Markdown summary appended to $GITHUB_STEP_SUMMARY. Active only when GITHUB_ACTIONS=true and stderr is not a TTY. JUnit follow-up filed as TASK-2338.
<!-- SECTION:NOTES:END -->
