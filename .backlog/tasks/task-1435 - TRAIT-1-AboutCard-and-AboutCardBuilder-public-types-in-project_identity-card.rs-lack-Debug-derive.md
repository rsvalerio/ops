---
id: TASK-1435
title: >-
  TRAIT-1: AboutCard and AboutCardBuilder public types in
  project_identity/card.rs lack Debug derive
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:33'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - trait
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:15,172`

**What**: The two public types in this module — `AboutCard` (line 15, `#[non_exhaustive] pub struct`) and `AboutCardBuilder` (line 172, only `#[derive(Default)]`) — both omit `Debug`. Their field types (`Option<String>`, `Vec<(String, String)>`) are all `Debug`, so the derive is mechanical and zero-cost.

**Why it matters**: Violates Rust API guideline C-DEBUG. Downstream extensions / CLI command crates that wrap an `AboutCard` field in a `#[derive(Debug)]` struct must hand-roll a `impl Debug`, and `tracing::debug!(card = ?card, ...)` does not compile for the about-card extension API. `AboutCardBuilder` has the same problem and breaks builder/built-type symmetry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutCard and AboutCardBuilder both derive Debug (and Clone where field types allow)
- [ ] #2 cargo build -p ops-core --all-features passes with the new derives
- [ ] #3 tracing::debug!(card = ?card) compiles in a downstream crate
<!-- AC:END -->
