---
id: TASK-0455
title: 'API-9: RunnerEvent enum is public but lacks #[non_exhaustive]'
status: To Do
assignee:
  - TASK-0537
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/events.rs:40`

**What**: `pub enum RunnerEvent` is exported as public API and crosses crate boundaries (runner ↔ display ↔ CLI), but neither the enum nor any struct-like variant carries `#[non_exhaustive]`. Adding a variant or field is a SemVer break for any downstream match or struct literal.

**Why it matters**: Sister types (ExtensionInfo, Context, DataField, DataProviderSchema) all carry #[non_exhaustive]. RunnerEvent is the most-changed event surface in the runner.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 RunnerEvent is annotated #[non_exhaustive] and each struct-like variant that may grow new fields is annotated similarly or documented as exhaustive
- [ ] #2 Existing match sites inside this workspace are updated to a wildcard arm or a TODO-noted exhaustive arm without warnings
<!-- AC:END -->
