---
id: TASK-2754
title: 'API-14: collect_coverage doc comment is attached to recover_partial_report'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2854'
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions-rust/test-coverage/src/parse.rs:collect_coverage'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:327-394`

**What**: The doc block written for `collect_coverage` (lines 327-350: "Run `cargo llvm-cov` and flatten its JSON output into per-file records", the `--no-fail-fast` soft-fail rationale, the `deadline` parameter) sits directly above the doc block for `recover_partial_report` (lines 351-364) with no item between them. Rustdoc merges both into one doc comment on `recover_partial_report`, whose summary paragraph therefore describes a different function and whose `deadline` paragraph names a parameter it does not take. `collect_coverage` itself (line 394) is left with no doc comment at all.

**Why it matters**: The crate's main collect entry point is undocumented, and the private helper's docs are misleading (wrong summary, nonexistent parameter). Anyone reading `recover_partial_report` in an IDE hover sees the wrong contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The collect_coverage doc block (summary, soft-fail rationale, deadline paragraph) is moved onto collect_coverage
- [ ] #2 recover_partial_report's doc comment describes only recover_partial_report and mentions no parameter it does not take
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.19 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
