---
id: TASK-2085
title: 'UNSAFE-12: ops-theme contains no unsafe but does not forbid it mechanically'
status: Done
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - unsafe
dependencies: []
parent_task_id: 'TASK-2245'
modified_files:
  - crates/theme/Cargo.toml
priority: low
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/Cargo.toml:21`

**What**: `ops-theme` contains zero `unsafe` code (verified by grep over `src/`), but neither its `[lints]` table nor the inherited `[workspace.lints.rust]` sets `unsafe_code = "forbid"`. The workspace lints table (`Cargo.toml` at repo root) carries `unsafe_op_in_unsafe_fn = "warn"` but no `unsafe_code` policy, and several sibling crates (core, runner, cli, backlog) do use `unsafe`, so a workspace-wide forbid is not the right shape — this is a per-crate lint.

**Why it matters**: `forbid` (not `deny`) makes "this crate contains no unsafe" a build-enforced fact rather than a property a reviewer re-derives, and it cannot be lifted by a later well-meaning `#[allow(unsafe_code)]` on a module. Per UNSAFE-12, most member crates in a workspace should carry it so the crates that genuinely need unsafe remain a short, visible list. This crate parses untrusted subprocess output (ANSI grammar in `style/strip.rs`); it is exactly the crate where a future "just one unsafe block for speed" PR should be a compile error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 crates/theme/Cargo.toml adds unsafe_code = "forbid" under a [lints.rust] section inheriting the rest from the workspace
- [x] #2 cargo check -p ops-theme still succeeds (no macro expansions in this crate emit unsafe tokens)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2245: added `#![forbid(unsafe_code)]` to crates/theme/src/lib.rs crate root. AC#1 substitution: the literal `[lints.rust]` table route is not satisfiable alongside ARCH-11 — Cargo cannot merge a per-crate lints table with `[lints] workspace = true`, and dropping inheritance would silently lose the centralized pedantic/nursery deny policy. The crate-root attribute enforces the same build-level forbid (cannot be lifted by a scoped allow) while keeping workspace lints intact; intent satisfied. AC#2 verified: cargo check -p ops-theme clean.
<!-- SECTION:NOTES:END -->
