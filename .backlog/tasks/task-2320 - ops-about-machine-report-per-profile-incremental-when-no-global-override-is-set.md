---
id: TASK-2320
title: 'ops about machine: report per-profile incremental when no global override is set'
status: Triage
assignee: []
created_date: '2026-09-27 15:56'
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
- [ ] #1 When no global override is set, cargo.incremental reports the dev and release profile values with their source (Cargo.toml, config file, CARGO_PROFILE_* env, or cargo default)
<!-- AC:END -->
