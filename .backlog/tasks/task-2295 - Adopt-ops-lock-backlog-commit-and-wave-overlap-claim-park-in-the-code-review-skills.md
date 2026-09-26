---
id: TASK-2295
title: 'Adopt ops lock, backlog commit and wave overlap/claim/park in the code-review skills'
status: Triage
assignee: []
created_date: '2026-09-26 19:05'
labels:
  - skills-integration
  - waves
dependencies: []
modified_files:
  - docs/backlog.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/backlog.md` (consumers live in the rsvalerio/ai dev-skills repo: code-review-run-wave, code-review-run-waves, code-review-triage)

**What**: TASK-2281/2285/2286/2289 added `ops lock`, `ops backlog commit`, `ops backlog wave overlap` and `ops backlog wave claim/park`, but the skills still run the shell they were meant to replace: `mkdir .git/code-review-merge.lock` + trap, the ~30-line backlog-lock/stage/diff/commit script, hand-computed Overlaps notes in triage, and `git worktree add -b` + separate status edit for claims.

**Why it matters**: until the skills switch, a killed runner still leaves a merge-lock directory behind with no owner info, and triage still does set arithmetic by hand.

**Origin**: discovered during TASK-2291 while fixing TASK-2281 (all four members share this follow-up).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 code-review-run-wave(s) serialize merges and backlog commits with ops lock and ops backlog commit
- [ ] #2 code-review-triage computes wave overlap with ops backlog wave overlap --json
- [ ] #3 code-review-run-wave claims/parks with ops backlog wave claim/park
<!-- AC:END -->
