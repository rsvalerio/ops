---
id: TASK-2526
title: 'FN-1: collect_coverage_with is a ~76-line function mixing four policy arms'
status: To Do
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - fn
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/test-coverage/src/parse.rs:collect_coverage_with'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:380-457`

**What**: `collect_coverage_with` runs ~76 body lines mixing: temp-report setup and path conversion, the injected run, the non-zero-exit recovery (read report, breadcrumb warn on read failure, soft-fail predicate + warn + early return, hard-fail fall-through), the success-path stderr diagnostic logging, and the final read + parse + flatten. The non-zero-exit recovery block alone (lines ~404-440) is a self-contained ~37-line policy that could be one helper returning `Option<serde_json::Value>` (Some = recovered partial report).

**Why it matters**: FN-1: functions <=50 lines, single abstraction level. The soft-fail recovery is the crate's most subtle policy (it decides when a failed cargo run still yields data) and is currently buried mid-function rather than named and independently readable.

<!-- scan confidence: measured at source (380-457, body excluding doc comment) -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 collect_coverage_with's body is <=50 lines with the non-zero-exit soft-fail recovery extracted into a named helper, without changing behaviour
<!-- AC:END -->
