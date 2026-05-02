---
id: TASK-0884
title: >-
  API-9: extensions-rust metadata public wrappers (Metadata, Package,
  Dependency, Target) lack #[non_exhaustive]
status: Done
assignee: []
created_date: '2026-05-02 09:37'
updated_date: '2026-05-02 11:05'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:102,203,326,334,...`

**What**: `Metadata`, `Package<'a>`, `Dependency<'a>`, `Target<'a>`, and `DependencyKind` are all `pub` and re-exported from the crate root (`pub use types::{Dependency, DependencyKind, Metadata, Package, Target};` in `lib.rs:10`). None carry `#[non_exhaustive]`. Although the structs only expose accessor methods (no public fields) and the enum is `#[non_exhaustive]`-eligible because adding a kind would otherwise break exhaustive matches in downstream code.

**Why it matters**: Sibling `extensions-rust/cargo-update/src/lib.rs` already applies `#[non_exhaustive]` to `UpdateAction`, `UpgradeEntry`, `CargoUpdateResult` etc.; consistency matters because these wrappers are advertised to extension authors via the schema. Without `#[non_exhaustive]` on `DependencyKind`, adding a future cargo `kind` (cargo has historically added new dependency kinds) becomes a breaking change. Adding it on the structs is precautionary against a future field-publication.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DependencyKind carries #[non_exhaustive]
- [ ] #2 Metadata, Package, Dependency, Target carry #[non_exhaustive] or an explicit comment justifying its absence
<!-- AC:END -->
