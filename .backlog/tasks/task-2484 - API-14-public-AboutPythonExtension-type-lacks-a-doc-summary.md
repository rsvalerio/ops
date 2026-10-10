---
id: TASK-2484
title: 'API-14: public AboutPythonExtension type lacks a doc summary'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - api
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions-python/about/src/lib.rs:AboutPythonExtension'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:36`

**What**: `AboutPythonExtension` — the crate's only public type, `#[non_exhaustive]`, constructed by the `impl_extension!` factory — has no `///` doc summary. The crate root carries good `//!` module docs, but the type itself renders undocumented in rustdoc and IDE hover.

**Why it matters**: API-14 makes the summary sentence mandatory on public items; it is what rustdoc lifts into the module index, so the crate's public surface currently shows an undescribed type. One or two lines stating what the extension registers (the `project_identity` and `project_units` Python providers) is enough.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutPythonExtension carries a /// doc summary of roughly 15 words stating what it provides
- [ ] #2 cargo doc -p ops-about-python renders the type with its summary in the crate index
<!-- AC:END -->
