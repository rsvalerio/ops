---
id: TASK-2502
title: 'READ-13: parse.rs docs and log messages carry TASK provenance tags'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/parse.rs:mod parse'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:3-7,15-19,98-99,136-139,177-180,203-208,232-235,265-267,317-327,337-359,370-374,386-392,421-423,449-451` and log-message strings at `192`, `301`, `434`

**What**: Nearly every doc comment opens with a rule/TASK tag and narrates prior implementations ("ARCH-1 / TASK-1559: lifted out of lib.rs", "extracted from the previous 106-line monolith", "TASK-0984: empty-key rows used to inflate project totals", the TASK-2068 budget essay on `llvm_cov_timeout`/`collect_coverage`). Worse, three `tracing::warn!` **message strings** bake internal task IDs into operator-visible output: `"TASK-0984: coverage file record has missing or non-string filename; skipping (llvm-cov schema drift?)"` (line 192), `"TASK-1021: coverage JSON contained duplicate filename rows across data[] exports; ..."` (line 301), and `"TASK-1057: cargo llvm-cov exited non-zero but the JSON report file is parseable; ..."` (line 434).

**Why it matters**: READ-13: documentation describes the end state, not the journey; TASK tags in docs go stale while looking authoritative. Task IDs in runtime log messages are additionally grep-noise for operators, who have no access to the internal backlog and see an unexplained "TASK-1021:" prefix on a diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No doc comment in parse.rs opens with a RULE-ID / TASK-XXXX tag or narrates a prior implementation
- [ ] #2 The three tracing::warn! message strings at the flatten/dedup/skip sites and the soft-fail site contain no TASK-XXXX reference while keeping their operational content
<!-- AC:END -->
