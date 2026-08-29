---
id: TASK-2055
title: 'FEAT: workspace.exclude does not drop members nested under an excluded path'
status: Done
assignee:
  - TASK-2064
created_date: '2026-08-29 13:24'
updated_date: '2026-08-29 17:42'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions-rust/about/src/members.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs` (`ExcludeSet::excludes`)

**What**: TASK-2040 taught `[workspace].exclude` the same single-`*` glob semantics `[workspace].members` has, so `exclude = ["crates/generated-*"]` now drops what it names. Two literal-path divergences from Cargo remain:

- Cargo excludes a path **and everything under it**: with `exclude = ["crates/foo"]`, a member listed as `crates/foo/bar` is excluded too. `ExcludeSet::excludes` compares whole strings, so the nested member survives.
- Exclude entries are not normalised, so `./crates/foo` does not match the resolved member `crates/foo` (the members side has the same gap — Cargo accepts `./` prefixes in both lists).

**Why it matters**: same failure mode TASK-2040 fixed, one level down — silent and biased toward over-counting. `module_count` (identity provider) and the `ProjectUnit` list (units / coverage providers) keep a crate the workspace excluded, with no warn explaining the divergence from `cargo metadata`. Rare shapes, so low priority, but the fix is bounded: compare path components rather than strings, after stripping a leading `./` on both sides.

**Origin**: discovered during TASK-2048 (code-review-plan-wave186) while fixing TASK-2040 — deliberately left out of that fix, which scoped itself to glob shapes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A member nested under a literal exclude entry (exclude = ["crates/foo"], member crates/foo/bar) is excluded, matching cargo metadata
- [x] #2 A leading ./ on either an exclude entry or a resolved member does not prevent a match
<!-- AC:END -->
