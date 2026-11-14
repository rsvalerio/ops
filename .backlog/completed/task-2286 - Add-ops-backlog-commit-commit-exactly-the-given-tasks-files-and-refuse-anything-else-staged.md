---
id: TASK-2286
title: 'Add ops backlog commit: commit exactly the given tasks'' files and refuse anything else staged'
status: Done
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 19:05'
labels:
  - feature
  - backlog
  - git
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2291'
modified_files:
  - crates/backlog/src/cmd/mod.rs
  - crates/cli/src/backlog_cmd.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog commit <task-id>… -m <msg>` stages exactly the files of the given tasks, and only those that actually changed. It aborts, leaving the index as it found it, if anything else is staged, and refuses to create an empty commit.

**Why**: `code-review-run-wave`'s bookkeeping step is about thirty lines of shell. It runs `git reset`, then loops over ids to resolve each file with `ops backlog task view --plain | sed -n '1s/^File: //p'`. It records only what `git diff --cached` shows as changed, compares the expected set against the staged set with `diff <(…) <(…)`, and guards against an empty commit. Every line exists because the naive version stages another wave's work or fails on an unchanged task file.

**Used by**: code-review-run-wave.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Only the listed tasks' changed files are committed
- [x] #2 Any other staged path aborts the commit and leaves the index as it was
- [x] #3 No changed task files means no commit and a non-zero exit with a clear message

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented crates/backlog/src/cmd/commit.rs: refuses before touching anything if any foreign path is staged; commits only changed task files via `git add` + `git commit --only -- <files>` (race-free against concurrent stagers); no changes -> error, no commit.
<!-- SECTION:NOTES:END -->
