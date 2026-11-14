---
id: TASK-2320
title: 'ops about machine: report per-profile incremental when no global override is set'
status: Done
assignee: []
created_date: '2026-09-27 15:56'
updated_date: '2026-09-27 17:17'
labels:
  - code-review-rust
  - feature
  - about
dependencies: []
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:resolve_cargo_settings`

**What**: `cargo.incremental` reports `CARGO_INCREMENTAL` / `CARGO_BUILD_INCREMENTAL` / `build.incremental`, and otherwise `{value: profile, source: default}`. It does not resolve what the profile itself says: `[profile.dev|release] incremental` in the workspace Cargo.toml or config, or `CARGO_PROFILE_<NAME>_INCREMENTAL`.

**Why it matters**: rust-make-build-fast's CACHE-1 check needs the effective value; a profile that turns incremental on or off is reported only as `profile`.

**Origin**: discovered during TASK-2314 while fixing TASK-2311 (the task sketch's "consider CARGO_PROFILE_* overrides").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When no global override is set, cargo.incremental reports the dev and release profile values with their source (Cargo.toml, config file, CARGO_PROFILE_* env, or cargo default)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `cargo.incrementalProfiles: {dev, release}` (null when CARGO_INCREMENTAL / CARGO_BUILD_INCREMENTAL / build.incremental applies). Each profile resolves CARGO_PROFILE_<NAME>_INCREMENTAL, then [profile.<name>] incremental in the nearest config file, then the workspace Cargo.toml (new `workspace_manifest`), then the cargo default (dev=true, release=false). The text report lists `dev` / `release` under `incremental`. `resolve_cargo_settings` now takes `Option<WorkspaceRoot>` (root + manifest) instead of `Option<&Path>`. Adding the field keeps the JSON schema backward-compatible (schema stays 1).
<!-- SECTION:NOTES:END -->
