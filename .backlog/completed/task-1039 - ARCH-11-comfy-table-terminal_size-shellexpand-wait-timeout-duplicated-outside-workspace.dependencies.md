---
id: TASK-1039
title: >-
  ARCH-11: comfy-table / terminal_size / shellexpand / wait-timeout duplicated
  outside [workspace.dependencies]
status: Done
assignee: []
created_date: '2026-05-07 20:44'
updated_date: '2026-05-08 06:49'
labels:
  - code-review-rust
  - arch
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/Cargo.toml:23-25`, `extensions-terraform/plan/Cargo.toml:11-14`, `extensions-rust/tools/Cargo.toml:13`

**What**: Several deps are pinned per-crate instead of being centralized in `[workspace.dependencies]`:

- `comfy-table = "7"` is duplicated in `crates/core/Cargo.toml` and `extensions-terraform/plan/Cargo.toml`.
- `terminal_size = "0.4"` is duplicated in `crates/core/Cargo.toml` and `extensions-terraform/plan/Cargo.toml`.
- `shellexpand = "3.1.2"` actually **diverges**: `crates/core` uses `default-features = false, features = ["base-0"]`, while `extensions-terraform/plan` uses the plain `"3.1.2"` form (default features on). Two crates pull in two different feature sets of the same dep.
- `wait-timeout = "0.2"` is already in `[workspace.dependencies]` (root `Cargo.toml:62`) but `extensions-rust/tools/Cargo.toml:13` redundantly inlines `wait-timeout = "0.2"` instead of `wait-timeout = { workspace = true }`.

**Why it matters**: ARCH-11 — diverging dep versions / feature sets across workspace crates. Today this compiles because semver-compatible versions resolve to the same crate, but the `shellexpand` feature divergence is a live correctness/build-size delta (the terraform crate silently pulls in `dirs` and other transitive deps that core deliberately disabled). Anything centralized in `[workspace.dependencies]` makes that drift a compile error rather than a stealth difference. This is the same class of finding as TASK-0413 / TASK-0585 / TASK-0616 / TASK-0633 / TASK-0646, applied to the remaining stragglers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add comfy-table, terminal_size, shellexpand, wait-timeout to [workspace.dependencies] (with the same default-features/features the strictest current consumer uses)
- [x] #2 Replace per-crate pins in crates/core, extensions-terraform/plan, and extensions-rust/tools with { workspace = true }
- [x] #3 cargo build --workspace --all-features compiles without diverging shellexpand feature sets
<!-- AC:END -->
