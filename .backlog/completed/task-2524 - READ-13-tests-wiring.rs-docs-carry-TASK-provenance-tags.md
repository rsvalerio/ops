---
id: TASK-2524
title: 'READ-13: tests/wiring.rs docs carry TASK provenance tags'
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
  - extensions-rust/test-coverage/src/tests/wiring.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/wiring.rs:mod wiring'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/wiring.rs:31-32,50-52`

**What**: Test comments open with "SEC-25 / TASK-2054: stage through the same verified anchor `provide_via_ingestor` builds" and the `load_coverage_returns_record_count` doc opens with "READ-5 (TASK-0808): the public `load_coverage` returns the structured `LoadResult` so callers can act on `record_count` instead of treating the load as opaque" — restating the finding that drove the signature change.

**Why it matters**: READ-13: the doc duplicates `load_coverage`'s own `# Errors`/doc contract with a task tag attached; the enduring fact (returns a `LoadResult` with `record_count`) needs no provenance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No comment in tests/wiring.rs opens with a RULE-ID / TASK-XXXX tag

<!-- AC:END -->
