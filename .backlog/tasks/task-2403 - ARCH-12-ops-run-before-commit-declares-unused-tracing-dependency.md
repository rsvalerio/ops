---
id: TASK-2403
title: 'ARCH-12: ops-run-before-commit declares unused tracing dependency'
status: Done
assignee: []
created_date: '2026-10-04 14:16'
updated_date: '2026-10-04 15:59'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/run-before-commit/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'ARCH-12:extensions/run-before-commit/Cargo.toml:dependencies'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/Cargo.toml` (`[dependencies] tracing`)

**What**: `src/lib.rs` has no `tracing::` use and no `tracing` macro invocation. The `impl_extension!` and `impl_hook_wrappers!` expansions do not reference `tracing` either. The `tracing::warn!` calls live in `ops-hook-common` (`git_state.rs`, `install.rs`), which declares its own dependency. `cargo-machete` flags `tracing`; `linkme` is also flagged but is a false positive, because `impl_extension!` expands to `#[linkme::distributed_slice(...)]` in the caller crate.

**Why it matters**: A dead dependency widens the manifest surface and suggests logging that does not exist. It can mislead readers about where the clamp warning is emitted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tracing removed from [dependencies] of ops-run-before-commit
- [x] #2 cargo build and cargo test -p ops-run-before-commit pass; linkme retained

<!-- AC:END -->
