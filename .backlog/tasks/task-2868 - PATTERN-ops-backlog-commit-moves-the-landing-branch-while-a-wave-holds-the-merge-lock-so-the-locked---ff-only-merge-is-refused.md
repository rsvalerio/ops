---
id: TASK-2868
title: 'PATTERN: ops backlog commit moves the landing branch while a wave holds the merge lock, so the locked --ff-only merge is refused'
status: Triage
assignee: []
created_date: '2026-11-18 21:33'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
modified_files:
  - crates/backlog/src/cmd/commit.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/commit.rs`

**What**: The wave merge protocol holds `ops lock code-review-merge` across rebase -> integration `ops verify` -> `git merge --ff-only`, but bookkeeping commits (`ops lock code-review-backlog -- ops backlog commit ...`) take only the backlog lock. A concurrent wave's `chore(backlog)` commit therefore lands on the landing branch during the multi-minute integration verify, and the final `--ff-only` is refused ("Not possible to fast-forward"). The whole locked command (including a full verify) must then be re-run, and under steady fan-out traffic it can be refused repeatedly.

**Why it matters**: Wasted verify cycles and a livelock risk for fan-out runs; observed twice in a row while landing TASK-2842 (run-20261118-1).

**Origin**: discovered during TASK-2842 while landing the wave (members TASK-2701..2704).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A backlog bookkeeping commit cannot move the landing branch between a wave's integration verify and its --ff-only merge (e.g. ops backlog commit also serialises on code-review-merge, or the merge step rebases backlog-only commits without re-verifying)
<!-- AC:END -->
