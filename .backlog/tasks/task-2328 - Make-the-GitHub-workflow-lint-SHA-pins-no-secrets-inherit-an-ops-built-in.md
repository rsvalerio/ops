---
id: TASK-2328
title: 'Make the GitHub workflow lint (SHA pins, no secrets: inherit) an ops built-in'
status: To Do
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:24'
labels:
  - ci
  - ops-alignment
  - security
dependencies: []
parent_task_id: 'TASK-2334'
modified_files: []
priority: low
ordinal: 1000
dedup_key: 'ops-align:ops-lint-actions'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: the same policy is implemented three times: ops ci.yml's workflow-guard grep, forge's `ci/lint.sh pinned-actions`, and not at all in ai (its actions are tag-pinned).

**Why it matters**: one implementation every repo's `verify` can include.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A built-in (e.g. `ops lint-actions`) enforces full-SHA pins with a version comment and rejects `secrets: inherit`, exempting local and allow-listed refs
- [ ] #2 forge's pinned-actions check and ops's workflow-guard are replaced by it

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1 done (fceb23f2): `ops lint-actions` enforces full-SHA pins with a `# vX.Y.Z` comment, docker digests, rejects `secrets: inherit`; exempts ./ refs, `[lint_actions] allow` in .ops.toml and `--allow`. AC#2 half done: ops ci.yml workflow-guard now runs `ops lint-actions --allow rsvalerio/forge/` (01237b74). Remaining: replace forge`s `ci/lint.sh pinned-actions` with it — forge repo, out of this repo; needs re-triage.
<!-- SECTION:NOTES:END -->
