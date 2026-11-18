---
id: TASK-2694
title: 'MACRO-3: impl_extension! expansion assumes caller depends on linkme'
status: Done
assignee: []
created_date: '2026-11-18 18:28'
updated_date: '2026-11-18 22:11'
labels:
  - code-review-rust
  - MACRO
dependencies: []
parent_task_id: 'TASK-2844'
modified_files:
  - crates/extension/src/macros.rs
  - crates/extension/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'MACRO-3:crates/extension/src/macros.rs:impl_extension'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/macros.rs:351`, `crates/extension/src/macros.rs:410`

**What**: Both `factory:` arms of `impl_extension!` emit `#[linkme::distributed_slice($crate::EXTENSION_REGISTRY)]`, a bare `linkme::` path resolved in the *invoking* crate. Every extension crate that uses the factory form must therefore declare `linkme` as a direct dependency it never names in source, and each one silences cargo-machete for it (`ignored = ["linkme"]` in 21 extension `Cargo.toml` files: extensions/*, extensions-rust/*, extensions-go/about, extensions-java/about, extensions-node/about, extensions-python/about, extensions-terraform/about).

**Why it matters**: MACRO-3 (1): a macro expansion must not assume the caller's dependencies. The coupling leaks an implementation detail of the registry into every consumer manifest, forces a machete suppression per crate (which also hides a genuinely unused `linkme` later), and breaks with a confusing error if a new extension crate forgets the dependency or renames it.

**Suggested fix**: re-export from this crate, e.g. `#[doc(hidden)] pub mod __private { pub use linkme; }`, and emit `#[linkme::distributed_slice(...)]` via the re-exported path together with linkme's `#[linkme(crate = $crate::__private::linkme)]` attribute. Then drop `linkme` and its machete ignore from the extension crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 impl_extension! factory arms resolve linkme through a $crate-relative re-export, not a bare linkme:: path
- [x] #2 Extension crates no longer need a direct linkme dependency nor a cargo-machete ignore for it
- [x] #3 Workspace builds and extension registration tests pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.04 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
