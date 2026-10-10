---
id: TASK-2517
title: 'READ-13: tests/parse_edge.rs docs carry TASK provenance tags'
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
  - extensions-rust/test-coverage/src/tests/parse_edge.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/parse_edge.rs:mod parse_edge'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/parse_edge.rs:78-80,99-100,121-124,145-149`

**What**: Test docs open with bare task tags and narrate the fixes they guard: "TASK-1599: when multiple files have the same wrong-shape field ... Two inserts of the same key -> set grows by 1, not 2", "TASK-1600: ... the warn volume is covered by the DriftTracker pattern", and the 5-line "ERR-1 / TASK-0984" essay ("a missing or non-string filename used to coerce to '' and still get pushed ... The fix skips such records").

**Why it matters**: READ-13: "used to" narration and bare TASK tags are process artifacts; the enduring content (wrong-shape fields coerce to defaults with one warn per section/field pair; records without a filename are skipped) stands on its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No test doc in tests/parse_edge.rs opens with a RULE-ID / TASK-XXXX tag or narrates the pre-fix behaviour

<!-- AC:END -->
