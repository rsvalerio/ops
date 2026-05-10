---
id: TASK-0557
title: 'API-9: ops_cargo_toml::InheritanceError lacks #[non_exhaustive]'
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:02'
updated_date: '2026-04-29 06:14'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:13`

**What**: pub enum InheritanceError is re-exported from ops_cargo_toml::lib.rs:64 without #[non_exhaustive]. TASK-0435 closed the same gap on the surrounding struct types but missed this enum, which is the only error type the inheritance API surfaces.

**Why it matters**: Adding a new variant becomes a SemVer-breaking change for downstream match consumers — exactly what API-9 / TASK-0167 is designed to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add #[non_exhaustive] to InheritanceError
- [ ] #2 Verify thiserror-generated Display/Error still compile
<!-- AC:END -->
