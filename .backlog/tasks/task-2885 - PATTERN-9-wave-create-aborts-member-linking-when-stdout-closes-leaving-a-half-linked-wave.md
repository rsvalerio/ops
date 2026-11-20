---
id: TASK-2885
title: 'PATTERN-9: wave create aborts member linking when stdout closes, leaving a half-linked wave'
status: Triage
assignee: []
created_date: '2026-11-19 07:57'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/backlog/src/cmd/wave_create.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave_create.rs:124-138` (member rewrite loop)

**What**: The loop rewrites each member (parent_task_id + status) and then `writeln!(out, "Linked {id}")?`. If stdout is closed, for example `ops backlog wave create … | head -2`, the write fails with BrokenPipe and `?` returns before the remaining members are rewritten. The parent already lists every member in `dependencies`, so the wave is left half-linked: the leftover members stay in their old status (e.g. Triage) with no `parent_task_id`. `wave members` shows them under the parent anyway, and `code-review-run-wave` would then run a wave whose members disagree with their own task files.

**Why it matters**: a bulk, mostly-atomic operation becomes partial because of a cosmetic progress line. It was hit for real during the 2026-11-18 triage: waves TASK-2879 and TASK-2883 each lost their third member until the members were linked by hand.

**Origin**: discovered during code-review-triage on 2026-11-19. It is the same pattern fixed in create-coderabbit-tasks (record the state first, then report).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 all member rewrites complete before any progress output, or an output failure no longer stops the remaining rewrites; the output error is still returned
- [ ] #2 a test with a writer that fails after the first line asserts every member got its parent_task_id and status
<!-- AC:END -->
