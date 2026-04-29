---
id: TASK-0595
title: 'ERR-1: flatten_coverage_json silently drops all data[] entries past index 0'
status: Triage
assignee: []
created_date: '2026-04-29 05:18'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:157`

**What**: flatten_coverage_json calls `data.first()` and only flattens files from that one entry. cargo llvm-cov --json emits data as an array (one entry per export); future per-target merging produces multiple exports — entries 1..N silently discarded.

**Why it matters**: Silent under-reporting of coverage. Schema/version drift in llvm-cov produces a partial table reported as complete; percentages drift downward without operators noticing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 flatten_coverage_json iterates all data[] entries OR emits tracing::warn! when data.len() > 1
- [ ] #2 Existing tests pass; new test exercises 2-entry data array
<!-- AC:END -->
