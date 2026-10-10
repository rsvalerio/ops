---
id: TASK-2503
title: 'READ-13: subprocess.rs docs narrate TASK history and rejected alternatives'
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
  - extensions-rust/test-coverage/src/subprocess.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/subprocess.rs:mod subprocess'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/subprocess.rs:3-7,15-21,24-29,52-69,82-94,108-114,122-131`

**What**: Every item doc in the module opens with a rule/TASK provenance tag and narrates the change that produced it: the module doc tells the TASK-1559 lift-out story, `CARGO_LLVM_COV_TIMEOUT`'s doc says "CONC-9 / TASK-2068: this is no longer the whole story", `LLVM_COV_ARGS` carries a TEST-23 / TASK-1554 essay about the previous `include_str!`-grepping test, `llvm_cov_timeout`'s doc is a 17-line TASK-2056/2052 design journal ("an operator who tightens the budget to 60s got the coverage provider blocking ... a bound that labelled the stall instead of curing it"), `format_cargo_exit` and `check_llvm_cov_output` narrate TASK-1099/1057 prior shapes, and `missing_tool_hint` explains what operators "used to" see.

**Why it matters**: READ-13: "why we picked X over Y" essays and migration notes are process artifacts; they go stale on the next change while looking authoritative. The enduring facts (the deadline caps the ceiling; the marker distinguishes signal kills) survive as one-sentence docs without the history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No doc comment in subprocess.rs opens with a RULE-ID / TASK-XXXX tag, and none narrate prior implementations or rejected alternatives
<!-- AC:END -->
