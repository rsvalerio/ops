---
id: TASK-2308
title: 'ops about machine: merge array-valued cargo config keys (rustflags) across config layers'
status: Triage
assignee: []
created_date: '2026-09-26 20:36'
labels:
  - code-review-rust
  - feature
dependencies: []
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs`

**What**: `lookup`/`from_config` return the value from the nearest config layer only. Cargo merges config files, and for array-valued keys (`build.rustflags`, `target.<triple>.rustflags`, `target.'cfg(..)'.rustflags`) it concatenates the arrays across layers (higher-precedence values appended after lower ones) rather than letting the nearest file win. A repo `.cargo/config.toml` rustflags plus a `~/.cargo/config.toml` rustflags are therefore reported as the repo's flags alone.

**Why it matters**: `rust-make-build-fast` records these next to timings; a missed home-config flag (e.g. `-fuse-ld=mold`) makes two timings look comparable when they are not.

**Origin**: discovered during TASK-2304 while fixing TASK-2300.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Array-valued rustflags keys are concatenated across config layers in cargo's order, with every contributing source named
- [ ] #2 A test pins a two-layer rustflags merge
<!-- AC:END -->
