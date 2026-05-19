---
id: TASK-1511
title: >-
  TRAIT-1: CargoTomlExtension and CargoTomlProvider lack Debug derive despite
  all-Debug fields
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 19:57'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-rust/cargo-toml/src/lib.rs:132-134` (`pub struct CargoTomlExtension { root: Option<PathBuf> }`)
- `extensions-rust/cargo-toml/src/lib.rs:184-186` (`pub struct CargoTomlProvider { root: Option<PathBuf> }`)

**What**: Both `CargoTomlExtension` and `CargoTomlProvider` are public types with a single `Option<PathBuf>` field — trivially `Debug` — but neither derives `Debug`. Consumers that store a provider in a `Box<dyn DataProvider>` registry, or that want to log "registered extension X with root=Y" via `#[derive(Debug)]` on a wrapper, cannot. The DataProvider trait does not require `Debug`, but `TRAIT-1` mandates that public structs derive `Debug` whenever every field is `Debug`. Sibling extensions (e.g. `extensions-rust/about`) consistently derive `Debug` on their `Extension`/`Provider` types.

**Why it matters**: Library API consistency. The omission is invisible until a downstream caller writes `#[derive(Debug)]` on a struct that contains a `CargoTomlProvider` and gets a compile error pointing at a crate they don't own.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CargoTomlExtension derives Debug
- [ ] #2 CargoTomlProvider derives Debug
- [ ] #3 no downstream Debug impl is broken (run cargo check across workspace)
<!-- AC:END -->
