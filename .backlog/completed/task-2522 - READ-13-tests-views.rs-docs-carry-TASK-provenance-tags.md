---
id: TASK-2522
title: 'READ-13: tests/views.rs docs carry TASK provenance tags'
status: Done
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:51'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/tests/views.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/views.rs:mod views'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/views.rs:8-11,68-69,79-80,130-138,170-174,192-196`

**What**: Test docs open with rule/TASK tags: "SEC-12 (successor of the deleted `test_create_sql_validation!` macro, which pinned the deleted `read_json_auto` builder)" — narrating two deleted things; "PATTERN-1 / TASK-1603", "READ-6 / TASK-1934" (twice), and the "TEST-6 / TASK-2200" essay explaining why the fixture is staged via `write_atomic` "not a raw `std::fs::write` on a bare `entry_path`, which bypasses exactly the anchored ... staging the comment above claims to exercise".

**Why it matters**: READ-13: the tests' names and assertions carry the contract; the TASK tags, "successor of" macro history, and self-referential comment narration are process artifacts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No test doc in tests/views.rs opens with a RULE-ID / TASK-XXXX tag or narrates deleted macros/builders

<!-- AC:END -->
