---
id: TASK-1541
title: >-
  TRAIT-1: public Metadata / Package / Dependency / Target types do not derive
  or implement Debug
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:24'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - TRAIT
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:102-104, 310-315, 432-447, 505-510`

**What**: The crate's four public types are all `pub` and `#[non_exhaustive]` but none of them derive `Debug`:
- `pub struct Metadata { inner: Arc<serde_json::Value>, member_ids: OnceLock<HashSet<String>>, ... }` (line 102-118)
- `pub struct Package<'a> { inner: &'a serde_json::Value, metadata: &'a Metadata }` (line 312-315)
- `pub enum DependencyKind { Normal, Dev, Build }` does derive `Debug` (line 433) — good, included only for context.
- `pub struct Dependency<'a> { inner: &'a serde_json::Value }` (line 443-447)
- `pub struct Target<'a> { inner: &'a serde_json::Value }` (line 506-510)

`pub use types::{Dependency, DependencyKind, Metadata, Package, Target};` re-exports them from `lib.rs:10`, so they appear in the public crate API.

**Why it matters**: Without `Debug`, downstream consumers cannot use these types in `assert_eq!`, `dbg!`, `tracing::debug!(?pkg)`, or `#[derive(Debug)]` on any struct that embeds them. The Rust API Guidelines (C-DEBUG) treat `Debug` as mandatory on public types; the project's TRAIT-1 rule mirrors that. `Metadata` wraps an `Arc<Value>` whose `Debug` would dump the entire workspace JSON — implement `Debug` manually to show just the field counts (e.g. `Metadata { packages: 73, members: 12 }`); `Package`/`Dependency`/`Target` can derive `Debug` trivially since they hold a `&Value`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Metadata, Package, Dependency, and Target implement Debug
- [ ] #2 Metadata's Debug impl does not dump the full Arc<Value> payload; it shows summary counts (e.g. workspace_root, package count)
- [ ] #3 Package/Dependency/Target Debug impls are derived or hand-written and produce stable, non-truncating output for the common fields
<!-- AC:END -->
