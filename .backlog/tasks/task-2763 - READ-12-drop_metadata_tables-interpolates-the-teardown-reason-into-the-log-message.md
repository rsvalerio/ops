---
id: TASK-2763
title: 'READ-12: drop_metadata_tables interpolates the teardown reason into the log message'
status: To Do
assignee: []
created_date: '2026-11-18 18:42'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'READ-12:extensions-rust/metadata/src/ingestor.rs:drop_metadata_tables'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:288`

**What**: `drop_metadata_tables` logs `tracing::warn!(error = %e, "failed to drop metadata_raw after {reason}; the next run may replay this failure")`. The `reason` value (one of four `&'static str` call-site labels) is interpolated into the message text instead of being recorded as a named field.

**Why it matters**: READ-12 asks for named structured fields and a constant message template. Because the reason is part of the message, a subscriber or collector cannot filter or group teardown failures by cause. The four call sites also produce four distinct message strings for a single event type.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The teardown-failure warn records reason as a named field (e.g. reason = reason) and uses a constant message
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.18 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
