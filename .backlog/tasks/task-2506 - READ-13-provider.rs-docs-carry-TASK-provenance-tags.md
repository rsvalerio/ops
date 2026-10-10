---
id: TASK-2506
title: 'READ-13: provider.rs docs carry TASK provenance tags'
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
  - extensions-rust/test-coverage/src/provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/provider.rs:mod provider'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/provider.rs:3,25-29,57-60`

**What**: The module doc narrates the TASK-1559 lift-out ("ARCH-1 / TASK-1559: lifted out of `lib.rs`"), the `schema()` field list carries a DUP-3 / TASK-1555 essay explaining that adding a metric "flows through one struct edit + this list" and that the projection and builder "no longer need parallel edits", and `query_coverage_files`'s doc opens with "DUP-3 / TASK-1555" plus a TASK-1610 note about the pre-by-name-binding behavior.

**Why it matters**: READ-13: the docs narrate the migration that produced the current shape rather than the current contract. What endures (the schema mirrors `CoverageRow`; column binding is by name so reordering errors loudly) is one sentence each, without the TASK tags.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No doc comment in provider.rs opens with a RULE-ID / TASK-XXXX tag or narrates the pre-refactor shape

<!-- AC:END -->
