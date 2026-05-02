---
id: TASK-0879
title: >-
  API-9: LoadResult is pub and #[non_exhaustive] but its fields are public, with
  #[allow(dead_code)]
status: Triage
assignee: []
created_date: '2026-05-02 09:24'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:13-30`

**What**: LoadResult { pub source_name, pub record_count } plus non_exhaustive - the non_exhaustive only blocks struct-init from out-of-crate. Out-of-crate code can still freely read both fields and pattern-match exhaustively at the value level. The #[allow(dead_code)] on LoadResult::success suggests the constructor is not actually used out-of-crate.

**Why it matters**: API surface that compiles for the wrong reason - dead_code allowances are a smell that the abstraction is in the wrong half of the privacy boundary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide: either remove #[allow(dead_code)] and use the constructor in a real call site, or remove the public fields and provide source_name(&self) -> &str / record_count(&self) -> u64 accessors
- [ ] #2 Update doc comment to describe the chosen shape
<!-- AC:END -->
