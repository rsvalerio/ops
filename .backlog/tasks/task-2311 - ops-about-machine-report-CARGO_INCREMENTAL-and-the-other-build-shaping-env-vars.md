---
id: TASK-2311
title: 'ops about machine: report CARGO_INCREMENTAL and the other build-shaping env vars'
status: To Do
assignee: []
created_date: '2026-09-27 15:20'
updated_date: '2026-09-27 15:25'
labels:
  - feature
  - about
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2314'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops about machine --json` resolves `jobs`, `rustcWrapper`, `targetDir`, `linker` and `rustflags`, each with its source layer, but not incremental compilation. `CARGO_INCREMENTAL`, and `[profile.*] incremental` via the build profile, decide whether sccache can cache workspace crates at all. On the eval machine `CARGO_INCREMENTAL=0` was set in the environment, and nothing in the report showed it.

**Why it matters**: the `rust-make-build-fast` skill's CACHE-1 check asks why sccache misses. Its first question is whether incremental compilation is on. The skill currently reads the `incremental` reason out of sccache's `not_cached` map instead, which only works when there were compilations to count. It has no way to read the setting itself without a shell `env` probe, which the skill is not allowed to run.

**Sketch**: add `cargo.incremental` `{value, source}` (from `CARGO_INCREMENTAL` or `build.incremental` in config, otherwise `default`). Consider also `CARGO_BUILD_JOBS` / `CARGO_PROFILE_*` overrides, if they are not already folded into `jobs`.

Source: re-eval of `rust-make-build-fast` on ops and dbsec, 2026-09-27.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `ops about machine --json` reports cargo.incremental with its value and source (env, config file, or default)
- [ ] #2 A test pins the env and config precedence
<!-- AC:END -->
