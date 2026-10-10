---
id: TASK-2528
title: 'FN-1: flatten_coverage_json is a ~69-line function still above the 50-line threshold'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:44'
labels:
  - code-review
  - fn
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/test-coverage/src/parse.rs:flatten_coverage_json'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:237-306`

**What**: `flatten_coverage_json` runs ~69 body lines: data-array validation + empty check + multi-export warn, a `file_arrays` collection with per-entry context, capacity pre-sizing of `records` and `filename_to_idx`, the flat_map loop with `build_record` + `dedup_push` and skipped-count accounting, two post-loop summary warns, and the final `serde_json::to_value`. The earlier TASK-1553 refactor extracted `build_record` and `dedup_push`, but the function remains above the FN-1 threshold.

**Why it matters**: FN-1: functions <=50 lines, one abstraction level per function. The remaining extractions are natural: the loop + counters block (build records with dedup, returning `(records, skipped_count, duplicate_count)`) and the post-loop warn emission.

<!-- scan confidence: measured at source (237-306, body excluding doc comment) -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 flatten_coverage_json's body is <=50 lines with helpers at a single abstraction level (e.g. records-loop extraction), without changing behaviour

<!-- AC:END -->
