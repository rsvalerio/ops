---
id: TASK-2514
title: 'READ-13: tests/parse.rs docs carry TASK provenance tags'
status: Done
assignee: []
created_date: '2026-10-10 15:32'
updated_date: '2026-10-10 21:51'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/tests/parse.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/parse.rs:mod parse'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/parse.rs:41-43,64-66,82-88,125-126`

**What**: Test docs open with "ERR-1 / TASK-0595" (twice, each narrating that "the previous shape silently dropped data[1..]" / "the earlier 'uses first only' behaviour silently dropped per-target merge exports") and "ERR-1 / TASK-1021" (narrating dedup policy provenance and "a future llvm-cov version").

**Why it matters**: READ-13: the tests' names already encode scenario and outcome; the docs' enduring content (multi-entry flatten, last-write-wins dedup) is one line each. The TASK tags and prior-behaviour narration are process artifacts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No test doc in tests/parse.rs opens with a RULE-ID / TASK-XXXX tag or narrates the pre-fix behaviour

<!-- AC:END -->
