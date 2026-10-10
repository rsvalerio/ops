---
id: TASK-2531
title: 'FN-1: run_wave_claim spans ~104 lines, mixing claim, undo, and edit-failure recovery inline'
status: To Do
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/wave_claim.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/backlog/src/cmd/wave_claim.rs:run_wave_claim'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave_claim.rs:61` (run_wave_claim, ~104 raw lines to line 165)

**What**: run_wave_claim exceeds the 50-line FN-1 threshold 2x. It mixes: option validation and worktree-path derivation, preflight refusals (branch exists, worktree taken), the `git worktree add -b` claim, the status edit, and the multi-branch undo-or-report recovery block (lines 133-156). The recovery logic alone is a named-procedure-sized unit.

**Why it matters**: The undo path is the safety net of the command ("a refused claim never leaves a task marked in progress") and it is the hardest part to read at the deepest nesting. Extracting derivation, preflight, and undo-recovery as named helpers makes each verifiable on its own.

<!-- scan confidence: raw-line measurement including comments -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_wave_claim delegates worktree-path derivation, preflight refusals, and undo-recovery to named helpers and stays under ~50 lines
- [ ] #2 Claim exclusivity and undo-on-edit-failure behaviour is unchanged (existing tests pass)
<!-- AC:END -->
