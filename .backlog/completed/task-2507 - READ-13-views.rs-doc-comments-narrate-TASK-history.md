---
id: TASK-2507
title: 'READ-13: views.rs doc comments narrate TASK history'
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
  - extensions-rust/test-coverage/src/views.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/views.rs:mod views'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/views.rs:39-50`

**What**: `coverage_summary_view_sql`'s doc is a two-rule history essay: "READ-6 / TASK-1934: every non-percentage SUM is wrapped in COALESCE(..., 0). An ungrouped aggregate ... without the wrapper a consumer decoding ... gets a decode failure instead of a zero." followed by "SEC-12 / TASK-1864: returned as the gated CreateViewSql newtype; ... never a path-bearing read_json_auto statement" — narrating what the code used to do before those tasks.

**Why it matters**: READ-13: the enduring contract (counts COALESCEd so an empty table yields 0 not NULL; identifiers const-validated so nothing runtime-derived reaches the loader) is one sentence each; the TASK tags and prior-shape narration are process artifacts that go stale.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 coverage_summary_view_sql's doc states the COALESCE and const-validation contracts without RULE-ID / TASK-XXXX tags or prior-shape narration

<!-- AC:END -->
