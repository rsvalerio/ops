---
id: TASK-2104
title: 'UNSAFE-12: unsafe-free ops-extension does not mechanically forbid unsafe code'
status: Done
assignee: []
created_date: '2026-09-08 06:42'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - idioms
dependencies: []
parent_task_id: 'TASK-2245'
modified_files:
  - crates/extension/src/lib.rs
priority: low
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/lib.rs` (crate root), root `Cargo.toml` [workspace.lints.rust]

**What**: ops-extension contains no `unsafe` and nothing in its lint config says so mechanically. The workspace lints table (root Cargo.toml, centralised per ARCH-11) does not include `unsafe_code = "forbid"`, and `[lints] workspace = true` in the crate's Cargo.toml means a per-crate lints-table addition is not available — so today a well-meaning PR could add an `unsafe` block to this crate and nothing would push back.

**Why it matters**: UNSAFE-12 — a crate that does not need `unsafe` should carry `unsafe_code = "forbid"` so "does this crate contain unsafe?" is a build-enforced fact rather than a reviewer's re-answered question. `forbid` (not `deny`) is the point: it cannot be lifted by a later scoped allow. The minimal, workspace-policy-compliant fix is `#![forbid(unsafe_code)]` in the crate root. Note the linkme caveat: if the `#[linkme::distributed_slice]` expansion (which the crate uses via impl_extension!'s factory arm) emits unsafe tokens into this crate under some configuration, the workspace-level route (deny in [workspace.lints.rust] plus scoped exceptions in the crates that genuinely hold unsafe: core, runner, cli, backlog) is the fallback; verify with cargo check -p ops-extension and cargo check -p ops-extension --features duckdb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 crates/extension/src/lib.rs gains an inner forbid(unsafe_code) attribute, or the root workspace lints table gains it with scoped, reasoned exceptions in the unsafe-holding crates
- [x] #2 cargo check -p ops-extension and cargo check -p ops-extension --features duckdb both pass
- [x] #3 Adding any unsafe block to the crate fails the build (forbid semantics, not deny)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2245: added `#![forbid(unsafe_code)]` to crates/extension/src/lib.rs crate root (the minimal ARCH-11-compliant route AC#1 names first). AC#2 verified: cargo check -p ops-extension and cargo check -p ops-extension --features duckdb both clean — the linkme distributed_slice caveat did not materialize (no unsafe tokens expanded into this crate). AC#3 verified by negative probe: an unsafe block fails cargo check with `error: usage of an unsafe block` (forbid semantics; probe removed afterwards).
<!-- SECTION:NOTES:END -->
