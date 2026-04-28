---
id: TASK-0433
title: >-
  ERR-1: RustUnitsProvider/RustCoverageProvider silently swallow
  load_workspace_manifest errors
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 04:42'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-rust/about/src/units.rs:25-28`
- `extensions-rust/about/src/coverage_provider.rs:21`

**What**: Both providers convert any load_workspace_manifest failure into an empty result without logging. units.rs:27 does `Err(_) => return Ok(serde_json::to_value(Vec::<ProjectUnit>::new())?)` and coverage_provider.rs:21 does `load_workspace_manifest(ctx).ok()`. The same crate's RustIdentityProvider::provide (identity/mod.rs:48) propagates the error correctly, so behavior is asymmetric.

**Why it matters**: A genuine failure (canonicalize error, malformed Cargo.toml, missing manifest, IO error) renders identically to a non-Rust project — operators have no signal that units/coverage are blank because the manifest could not be loaded. Sibling fix TASK-0376 added warn-logs to all DuckDB query callsites in this same module precisely to avoid this class of silent fallback; the manifest path was missed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit at minimum tracing::warn! (or debug! if manifest absent is expected) including the underlying error before returning the empty fallback in both providers
- [ ] #2 Differentiate no manifest / not a Rust project (silent or debug) from manifest read/parse error (warn) by inspecting the error kind, mirroring read_crate_metadata (units.rs:86-113)
<!-- AC:END -->
