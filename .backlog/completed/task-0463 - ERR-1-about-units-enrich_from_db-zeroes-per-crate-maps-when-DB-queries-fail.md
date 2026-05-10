---
id: TASK-0463
title: 'ERR-1: about/units enrich_from_db zeroes per-crate maps when DB queries fail'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 18:47'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:76`

**What**: query_crate_loc / query_crate_file_count failures call .unwrap_or_else(|e| { warn; Default::default() }) returning empty HashMap. The subsequent loop assigns `unit.loc = locs.get(...).copied()` (None for every unit). A transient lock error or schema miss silently presents every crate as having no LOC/file data, indistinguishable from "no tokei data".

**Why it matters**: Caller cannot tell a query error from legitimately-empty workspace. Combined with debug-level visibility, this is silent partial data loss in the UI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When query_crate_loc fails, unit's .loc is left at provider-supplied value (do not overwrite Some(_) with None); warning includes a unit-count summary not just an opaque {e:#}
- [ ] #2 Test simulates a poisoned mutex on db and asserts pre-existing unit.loc / unit.file_count values are preserved across enrich_from_db
<!-- AC:END -->
