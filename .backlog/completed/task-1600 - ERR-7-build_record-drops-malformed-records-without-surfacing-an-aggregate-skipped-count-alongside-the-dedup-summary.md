---
id: TASK-1600
title: >-
  ERR-7: build_record drops malformed records without surfacing an aggregate
  skipped-count alongside the dedup summary
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 08:49'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:138-155`

**What**: When a record is dropped (missing/non-string filename), only a per-occurrence warn fires. `flatten_coverage_json` reports `duplicate_count` as an aggregate but no comparable "records skipped due to schema drift" total. Operators have to count warn lines by hand to gauge ingest pollution.

**Why it matters**: TASK-0984's intent (aggregates stay honest) is only monitorable via per-line breadcrumbs. A summary makes it dashboard-actionable and symmetric with the existing dedup summary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 build_record returns a result the caller can tally (Option<CoverageRow> + &mut skipped, or Result<CoverageRow, SkipReason>)
- [ ] #2 Emit a single summary tracing::warn! after the loop when skipped > 0, mirroring duplicate_count
- [ ] #3 Test feeds three malformed file records and asserts exactly one summary warn fires with skipped = 3
<!-- AC:END -->
