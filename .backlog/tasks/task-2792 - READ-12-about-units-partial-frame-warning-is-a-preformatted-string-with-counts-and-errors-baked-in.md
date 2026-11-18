---
id: TASK-2792
title: 'READ-12: about units partial-frame warning is a preformatted string with counts and errors baked in'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/units.rs
priority: low
ordinal: 1000
dedup_key: 'READ-12:extensions/about/src/units.rs:enrich_from_db'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:268-280` (`enrich_from_db`)

**What**: The consolidated partial-failure warning formats `unit_count`, the failed field list and the joined `field: error` detail into the message text (`"about/units: rendered frame is partial across {unit_count} units; ... [{fields}]: {detail}"`), with `detail` pre-built by `format!` in a loop.

**Why it matters**: The message differs on every occurrence, so it cannot be grouped or filtered, and `unit_count` / failed fields are not addressable as fields. READ-12 wants a constant template with `unit_count`, `fields` and `errors` as named fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The warning has a constant message and records unit_count, the failed query names and their errors as named fields
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
