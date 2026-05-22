---
id: TASK-1625
title: >-
  API-2: SourceName/WorkspaceRoot newtypes expose pub field, defeating
  swap-safety guard
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:12'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:55-59`

**What**: `pub struct SourceName<'a>(pub &'a str)` and `pub struct WorkspaceRoot<'a>(pub &'a OsStr)` expose their inner field as `pub`. The newtype guard (introduced by API-2/TASK-0912 to make swapping the two adjacent `&str` parameters of `DataSourceMetadata::new` a compile error) only applies at the constructor call site. Any code path that destructures the tuple struct (`let SourceName(s) = name;`) or constructs via positional field access still mixes the two freely. The doc comment claims "Swap is now a compile error" but the field-level pub leaks the invariant.

**Why it matters**: The two halves are the primary key for `data_sources`. Silently writing rows under the wrong key produces duplicate ingest records and divergent checksums no future run can reconcile — exactly the scenario the newtype was meant to prevent. Hardening the encapsulation closes the leak without changing call sites that already use `SourceName(x)` constructor form.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inner field of SourceName and WorkspaceRoot is made private
- [ ] #2 Constructor (e.g. SourceName::new(&str)) and accessor (e.g. as_str()) provided
- [ ] #3 Call sites (ingestor.rs:262-263 and tests) compile against the new API
<!-- AC:END -->
