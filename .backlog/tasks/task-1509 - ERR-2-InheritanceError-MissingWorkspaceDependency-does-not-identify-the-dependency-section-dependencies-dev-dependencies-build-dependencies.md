---
id: TASK-1509
title: >-
  ERR-2: InheritanceError::MissingWorkspaceDependency does not identify the
  dependency section (dependencies / dev-dependencies / build-dependencies)
status: To Do
assignee:
  - TASK-1642
created_date: '2026-05-18 19:15'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:17-21, 28-40, 143-153`

**What**: `InheritanceError::MissingWorkspaceDependency { name }` carries only the dependency name. `resolve_inheritance` calls `resolve_deps_inheritance` three times — for `dependencies`, `dev_dependencies`, and `build_dependencies` — and any of them can produce this error, but the variant gives the operator no way to know which section the offending entry lived in. The error surfaces eventually as e.g. `parsing Cargo.toml ... resolving workspace inheritance: dependency 'foo' not found in workspace.dependencies` (because lib.rs:235 wraps it with `.context("resolving workspace inheritance")`), and the user has to grep the manifest by hand to discover that `foo` was actually under `[dev-dependencies]`.

In a workspace member with the same crate name appearing in two sections (say `serde = { workspace = true }` in `[dependencies]` plus `serde = { workspace = true }` in `[build-dependencies]`), and only one of them missing from `[workspace.dependencies]`, the message is actively misleading.

**Why it matters**: ERR-2 / ERR-7. The error variant is the only structured context callers ever see; dropping the section name forces consumers (notably the about/units stack that surfaces this through reports) to fall back to substring matching on the dep name, with no way to disambiguate. Adding a `section: &'static str` (`"dependencies"`, `"dev-dependencies"`, `"build-dependencies"`) to the variant is a one-field change that materially shortens the debugging loop and is forward-compatible thanks to `#[non_exhaustive]`.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 InheritanceError::MissingWorkspaceDependency carries the dependency section name (dependencies / dev-dependencies / build-dependencies) in addition to the dep name
- [ ] #2 Display impl renders the section in the error message so operators no longer have to grep the manifest to locate the offending entry
- [ ] #3 resolve_deps_inheritance call sites pass the appropriate section literal
<!-- AC:END -->
