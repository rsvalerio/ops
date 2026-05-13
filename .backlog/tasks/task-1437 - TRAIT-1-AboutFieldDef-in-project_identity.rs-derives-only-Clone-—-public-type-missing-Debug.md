---
id: TASK-1437
title: >-
  TRAIT-1: AboutFieldDef in project_identity.rs derives only Clone — public type
  missing Debug
status: To Do
assignee:
  - TASK-1452
created_date: '2026-05-13 18:33'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - trait
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity.rs:295`

**What**: `#[derive(Clone)] pub struct AboutFieldDef { id: &'static str, label: &'static str, description: &'static str }` exposes a public field-metadata type with only `Clone`. Every other public type in `ops-core::project_identity` derives `Debug, Clone, Default, Serialize, Deserialize` — this one type is the lone exception.

**Why it matters**: Rust API guideline C-DEBUG. Extension authors collecting `Vec<AboutFieldDef>` to log the resolved field set cannot `tracing::debug!(?defs)` and must format each entry by hand. The asymmetry within the module is also a maintainability cue: anyone copying a sibling struct as a template gets the derives, but this one diverges silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutFieldDef derives Debug alongside Clone
- [ ] #2 cargo build -p ops-core --all-features passes
- [ ] #3 tracing::debug!(?defs) over a Vec<AboutFieldDef> compiles
<!-- AC:END -->
