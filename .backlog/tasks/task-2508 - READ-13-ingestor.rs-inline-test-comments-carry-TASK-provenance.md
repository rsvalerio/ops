---
id: TASK-2508
title: 'READ-13: ingestor.rs inline test comments carry TASK provenance'
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
  - extensions-rust/test-coverage/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/ingestor.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/ingestor.rs:49-50,56-64,68-69`

**What**: The inline `#[cfg(test)]` module carries TASK-tag narration: two "SEC-25 / TASK-2054: stage through the same verified anchor `provide_via_ingestor` builds" comments, and a 9-line DUP-3 / TASK-1562 essay on `coverage_load_with_sample_data` explaining why this test keeps its own single-file fixture instead of the shared one, narrating what "the shared fixture deliberately ships" and which value "used to" be rewritten.

**Why it matters**: READ-13: test comments describing fixture choices in terms of the TASK that produced them are process artifacts. The enduring fact (this test owns the single-file `lines_count = 100` round-trip, so it does not use the two-file shared fixture) needs no TASK tags or history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No comment in ingestor.rs's test module opens with a RULE-ID / TASK-XXXX tag
<!-- AC:END -->
