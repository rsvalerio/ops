---
id: TASK-0662
title: >-
  ERR-1: enrich_from_db query_project_loc overwrites provider-supplied loc with
  Some(0)
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 18:30'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:150-152`

**What**: `query_project_loc` result `Ok(loc)` unconditionally writes `identity.loc = Some(loc)`, overwriting a provider-supplied `loc` with `Some(0)` when the `tokei_files` table does not exist (`query_project_scalar` returns 0 for missing table).

**Why it matters**: A stack `project_identity` provider that already populated `loc` from another source gets clobbered to `Some(0)` whenever DuckDB lacks tokei data, rendering "0 loc" instead of the real value. Sibling branches at lines 154-155 (file_count), 159-164 (dependency_count), and 167-171 (coverage_percent) all guard with `> 0`; only `loc` is missing the guard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Apply a > 0 (or is_some_and-style) guard so query_project_loc == 0 does not overwrite a non-None provider-supplied loc
- [ ] #2 Add a regression test where identity.loc = Some(N) survives an enrich_from_db invocation against an empty DuckDB
<!-- AC:END -->
