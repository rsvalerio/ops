---
id: TASK-2539
title: 'READ-13: commit module docs narrate the shell guard this command replaced'
status: To Do
assignee: []
created_date: '2026-10-10 15:37'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/commit.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/backlog/src/cmd/commit.rs:commit'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/commit.rs:1-18` (module docs)

**What**: The module doc mixes enduring behaviour (the numbered 4-step contract, the `--only` race-closing property) with journey narration: "This command replaces the shell the wave runner used to guard against that" and the framing of the problem in terms of what "a bookkeeping commit built with `git add .backlog`" used to sweep up. The replaced shell and the old sweep behaviour are process artifacts.

**Why it matters**: Per READ-13 the end state is: "commit exactly the named tasks' files, and nothing else, even with concurrent writers staging between the steps" — which the doc's own first line and step list already state. The "replaces the shell" sentence would be deleted verbatim by a reader who joined after the decision; it belongs in the PR description/ADR.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module docs keep the 4-step contract and the --only concurrency property, drop the replaced-shell narration
<!-- AC:END -->
