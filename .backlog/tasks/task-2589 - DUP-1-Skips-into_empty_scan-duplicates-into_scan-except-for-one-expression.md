---
id: TASK-2589
title: 'DUP-1: Skips::into_empty_scan duplicates into_scan except for one expression'
status: Done
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:58'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2626'
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:extensions/tokei/src/lib.rs:Skips'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:310-318` (`into_empty_scan`) vs `:321-329` (`into_scan`)

**What**: The two `Skips` constructors are 8-line struct literals identical in every field except `records`: `Vec::new()` in `into_empty_scan` versus the `records` parameter in `into_scan`. `into_empty_scan` is a wrapper `into_scan` already expresses: its single call site (`scan_tokei`, `:247`) could be `return Ok(skips.into_scan(Vec::new()));`.

**Why it matters**: DUP-1 — near-identical blocks drift; a field added to `TokeiScan`/`Skips` accounting must be threaded through both constructors (and indeed every skip field is already written twice). The abstraction the duplicate avoids already exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Single constructor: into_empty_scan is removed and the empty-candidates path in scan_tokei calls into_scan(Vec::new())
- [x] #2 cargo test -p ops-tokei passes unchanged (pure refactor, no behavior change)

<!-- AC:END -->
