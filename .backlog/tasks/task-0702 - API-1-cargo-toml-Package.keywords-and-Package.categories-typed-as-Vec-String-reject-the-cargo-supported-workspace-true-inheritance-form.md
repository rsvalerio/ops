---
id: TASK-0702
title: >-
  API-1: cargo-toml Package.keywords and Package.categories typed as Vec<String>
  reject the cargo-supported '{ workspace = true }' inheritance form
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:27'
updated_date: '2026-04-30 11:50'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:142,146`

**What**: `Package.keywords` and `Package.categories` are declared `Vec<String>` (lines 141-146 of `types.rs`), while every other workspace-inheritable scalar/list (`version`, `edition`, `rust_version`, `description`, `documentation`, `homepage`, `repository`, `license`, `authors`) uses `InheritableField<...>` / `InheritableVec`. Cargo allows both `keywords = { workspace = true }` and `categories = { workspace = true }` in a package manifest — when ops_cargo_toml encounters such a member, `toml::from_str` errors and the entire `provide()` call fails (line 194-195 of `lib.rs` propagates the error via `with_context`).

**Why it matters**: Real-world workspaces that consolidate `keywords`/`categories` in `[workspace.package]` (cargo published support since 1.64 / 2022) silently break `ops about`, `ops deps`, and every consumer of the `cargo_toml` data provider on those projects. The fix is mechanical (switch to `InheritableVec`) but interacts with `resolve_package_inheritance`, which today does not propagate either field — see sibling finding.

<!-- scan confidence: candidates to inspect -->
- types.rs:140-142 (`keywords: Vec<String>`)
- types.rs:144-146 (`categories: Vec<String>`)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Package.keywords and Package.categories accept the '{ workspace = true }' shape without erroring at deserialisation
- [x] #2 resolve_package_inheritance substitutes keywords/categories from [workspace.package] when the field is in the Inherited state
- [x] #3 regression test pins a member crate that inherits both keywords and categories from [workspace.package]
<!-- AC:END -->
