---
id: TASK-0987
title: >-
  DUP-3: parse_package_metadata wrapper duplicated between node and python units
  providers
status: Triage
assignee: []
created_date: '2026-05-04 21:59'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-node/about/src/units.rs:231-239`
- `extensions-python/about/src/units.rs:114-126`

**What**: Both crates have a `parse_package_metadata(path, content)` shim that calls `ops_about::workspace::parse_package_metadata(path, content, |c| ...)` with a closure that does:
1. `serde_json::from_str::<PackageProbe>` (node) / `toml::from_str::<PackageProbe>` (python)
2. Map fields into `ops_about::workspace::PackageMetadata { name, version, description }`

The structural shape is identical; only the deserialiser and the projection-out-of-`project`-table indirection differ. The same DUP-3 sweep that produced TASK-0931 (manifest_cache) and TASK-0816 (pyproject parse-once) leaves this glue duplicated.

**Why it matters**: A future change to the metadata field set (e.g., adding a `keywords` or `license` field to the workspace card) requires touching every per-stack `PackageProbe` struct and its `From`-style projection. The shape is small enough today that an extraction may not pay off, but the pattern doubles cleanly: when a third stack (Go / Java / Rust) lands a workspace metadata probe, it will copy this shim again. Consider exposing a `ops_about::workspace::parse_package_metadata_with::<F>(path, content, deserialise: F)` that takes a function returning `(name, version, description)` directly so the per-crate `PackageProbe` and projection live next to the deserialiser only.

<!-- scan confidence: candidates to inspect — the duplication is real but small (8 lines each); the value is in the trend, not the line count -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-stack PackageProbe + projection lives next to the deserialiser in one place per crate, with no parallel shim function
- [ ] #2 Adding a new field to PackageMetadata requires changing one site per stack, not two
<!-- AC:END -->
