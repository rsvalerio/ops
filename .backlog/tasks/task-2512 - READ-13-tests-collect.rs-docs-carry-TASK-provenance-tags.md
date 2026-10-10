---
id: TASK-2512
title: 'READ-13: tests/collect.rs docs carry TASK provenance tags'
status: To Do
assignee: []
created_date: '2026-10-10 15:32'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/tests/collect.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/collect.rs:mod collect'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/collect.rs:3-7,47-48,61-64,82-85,109-112,138-140,154-163,183-185`

**What**: Every test doc in the file opens with rule/TASK tags: "TEST-6 / TASK-1938" (four times, narrating that a real run "is not a unit test"), "ERR-1 / TASK-1057, TEST-6 / TASK-1938", "ERR-1 / TASK-1557 + TASK-1597", "ERR-13 / TASK-1949", "DUP-1 / TASK-1929" with a paragraph on what "the test previously declared", and "READ-4 / TASK-1941" narrating a prior stdout-carrying shape.

**Why it matters**: READ-13: test docs should state scenario and expected outcome; the TASK tags and prior-shape narration are process artifacts. Sibling crates have already had these stripped in dedicated waves.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No test doc in tests/collect.rs opens with a RULE-ID / TASK-XXXX tag or narrates a prior test shape
<!-- AC:END -->
