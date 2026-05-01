---
id: TASK-0798
title: >-
  PERF-1: RustCoverageProvider::provide calls resolve_crate_display_name in a
  per-member loop, re-reading every Cargo.toml
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:01'
updated_date: '2026-05-01 06:58'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:71-85`

**What**: For every member that has coverage, the closure calls resolve_crate_display_name(member, cwd) which does std::fs::read_to_string + toml::from_str on member/Cargo.toml. With N members and the inner manifest already loaded once via load_workspace_manifest, this re-reads and re-parses every member manifest from disk on every ops about coverage invocation.

**Why it matters**: Workspace manifest already contains package metadata for member crates after resolve_inheritance, and the typed cache holds an Arc<CargoToml> for the workspace root. Reading per-member Cargo.toml is the cost the typed cache was designed to avoid (TASK-0558).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either cache per-member manifests in the same TYPED_MANIFEST_CACHE shape, or compute the display-name map up front (one pass over members) before the filter_map
- [ ] #2 resolve_crate_display_name is called at most once per member per provide() call
- [ ] #3 A unit/integration test asserts the read count remains O(N) members rather than 2N+
<!-- AC:END -->
