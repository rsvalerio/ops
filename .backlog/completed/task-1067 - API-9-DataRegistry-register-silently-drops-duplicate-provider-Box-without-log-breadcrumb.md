---
id: TASK-1067
title: >-
  API-9: DataRegistry::register silently drops duplicate provider Box without
  log breadcrumb
status: Done
assignee: []
created_date: '2026-05-07 21:18'
updated_date: '2026-05-08 06:23'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:170-177`

**What**: On collision the `Box<dyn DataProvider>` argument is dropped without notice; `duplicate_inserts` records only the name. A provider that opened a DB handle or file in its constructor leaks side-effects with no log at the drop site.

**Why it matters**: Silent provider replacement masks bugs where two extensions register under the same key, and any constructor side effects in the dropped provider go unreported. First-write-wins is the right policy; the missing signal is the issue.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tracing::debug! at the duplicate-drop site naming the dropped provider type or marker
- [x] #2 Document the Drop / first-write-wins semantics in the rustdoc
- [x] #3 Behavior remains first-write-wins (no functional change)
<!-- AC:END -->
