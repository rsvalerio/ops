---
id: TASK-2328
title: 'Make the GitHub workflow lint (SHA pins, no secrets: inherit) an ops built-in'
status: In Progress
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-29 17:14'
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

AC#2 prerequisite done in ops: forge's pinned-actions also covers its composite actions (actions/*/action.yml), which lint-actions did not scan, so swapping would have dropped that coverage. lint-actions now also scans composite action manifests (root action.yml/.yaml, and one dir deep under .github/actions/ and actions/), skipping symlinked files and dirs; against forge main it passes the same 17 files (8 workflows + 9 actions) as ci/lint.sh pinned-actions. Remaining, in forge, after the next ops release: bump mise.toml ops to it, add [lint_actions] allow = ["rsvalerio/forge/"] to forge .ops.toml, point verify's lint-pinned-actions at ops lint-actions, delete check_pinned_actions from ci/lint.sh, and update README rule 6 and the dependabot.yml comment.

ops side in rsvalerio/ops#81.

<!-- SECTION:NOTES:END -->
