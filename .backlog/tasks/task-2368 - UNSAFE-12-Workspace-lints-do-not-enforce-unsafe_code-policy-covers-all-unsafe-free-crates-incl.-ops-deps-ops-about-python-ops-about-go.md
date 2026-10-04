---
id: TASK-2368
title: 'UNSAFE-12: Workspace lints do not enforce unsafe_code policy (covers all unsafe-free crates, incl. ops-deps, ops-about-python, ops-about-go)'
status: Triage
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 14:31'
labels:
  - code-review-rust
  - unsafe
dependencies: []
modified_files:
  - Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'UNSAFE-12:Cargo.toml:workspace.lints.rust'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `Cargo.toml` (`[workspace.lints.rust]`, ~line 100); inherited by `extensions-rust/deps/Cargo.toml` via `[lints] workspace = true`

**What**: `ops-deps` (all of `extensions-rust/deps/src`) contains no `unsafe`, but neither the workspace lint table nor the crate root sets `unsafe_code = "forbid"` (grep for `unsafe_code` across Cargo.toml/clippy.toml finds nothing). `[workspace.lints.rust]` only sets `unsafe_op_in_unsafe_fn`, which is moot without unsafe.

**Why it matters**: The no-unsafe property is not enforced by the build; a later PR can add an `unsafe` block to this parser/report crate unnoticed. Workspace-wide fix (other crates that need unsafe opt out with `deny` plus scoped `#[expect]`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Set unsafe_code = "forbid" in [workspace.lints.rust] and confirm the whole workspace builds; if crates using impl_extension! (linkme #[link_section]) fail, use deny at the workspace level instead and let crates that need it carry the scoped #[allow]/#[expect(unsafe_code, reason = ...)] that crates/extension/src/macros.rs already emits
- [ ] #2 Any workspace crate that genuinely needs unsafe (e.g. libc kill in ops, tcgetattr in ops-runner, text.rs/config edit in ops-core) is listed explicitly with deny + scoped #[expect(unsafe_code, reason = ...)]

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Consolidated 2026-10-04: absorbs TASK-2357 (ops-about-python) and TASK-2363 (ops-about-go), which asked for the same per-crate lint. Note from TASK-2363: forbid is not possible in crates using impl_extension! because linkme expands to #[link_section] statics, so deny is the likely level for those crates.
<!-- SECTION:NOTES:END -->
