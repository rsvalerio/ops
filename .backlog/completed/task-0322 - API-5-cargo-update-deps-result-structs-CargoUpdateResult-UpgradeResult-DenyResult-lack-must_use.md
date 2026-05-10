---
id: TASK-0322
title: >-
  API-5: cargo-update / deps result structs (CargoUpdateResult, UpgradeResult,
  DenyResult) lack #[must_use]
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:54'
updated_date: '2026-04-25 13:19'
labels:
  - rust-code-review
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/cargo-update/src/lib.rs:33-50; extensions-rust/deps/src/parse.rs (UpgradeResult, DenyResult)

**What**: Public data-carrying structs returned from run_* helpers are not marked #[must_use].

**Why it matters**: Callers can silently drop the entire result — counts, update entries, or deny findings go unobserved.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 #[must_use] added to the structs or to the run_* functions returning them
- [ ] #2 cargo clippy passes with no new warnings
<!-- AC:END -->
