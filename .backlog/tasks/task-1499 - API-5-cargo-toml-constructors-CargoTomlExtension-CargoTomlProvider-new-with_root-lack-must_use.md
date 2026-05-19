---
id: TASK-1499
title: >-
  API-5: cargo-toml constructors CargoTomlExtension/CargoTomlProvider
  new/with_root lack #[must_use]
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 18:03'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:138`, `:144`, `:190`, `:195`

**What**: `CargoTomlExtension::new`, `CargoTomlExtension::with_root`, `CargoTomlProvider::new`, and `CargoTomlProvider::with_root` are pure constructors returning owned values but are not marked `#[must_use]`.

**Why it matters**: API-5 calls for `#[must_use]` on builders/constructors so dropping the return at the call site (e.g. `CargoTomlExtension::new();` with a stray semicolon during a refactor) raises a `clippy::must_use_candidate`-level warning instead of compiling silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All four constructors carry #[must_use]
- [ ] #2 cargo clippy --all-targets remains clean
<!-- AC:END -->
