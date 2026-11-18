---
id: TASK-2697
title: 'API-16: ExtensionInfo lacks Debug and Clone'
status: To Do
assignee: []
created_date: '2026-11-18 18:29'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2844'
modified_files:
  - crates/extension/src/extension.rs
priority: low
ordinal: 1000
dedup_key: 'API-16:crates/extension/src/extension.rs:ExtensionInfo'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:120-129`

**What**: `ExtensionInfo` is a public, `#[non_exhaustive]` plain-data snapshot (all fields `&'static str`, `ExtensionType`, `Option<Stack>`, `&'static [&'static str]`), yet it derives nothing: no `Debug`, `Clone`, `Copy`, `PartialEq`/`Eq`. Every sibling public data type in the crate (`DataField`, `DataProviderSchema`, `ExtensionType`, `CommandRegistry`, `Deadline`) derives at least `Debug` and `Clone`.

**Why it matters**: API-16 (C-COMMON-TRAITS): common traits should be implemented eagerly where semantics hold, because downstream crates cannot add them. Callers cannot `{:?}`-log an info snapshot, `assert_eq!` one in a test, derive `Debug` on a type that holds one, or copy it out of a borrow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ExtensionInfo derives Debug, Clone and, where all fields allow it, Copy, PartialEq and Eq
- [ ] #2 A public_api test exercises Debug/Clone on an ExtensionInfo built via Extension::info
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.04 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
