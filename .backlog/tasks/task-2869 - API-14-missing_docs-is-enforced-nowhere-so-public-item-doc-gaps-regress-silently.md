---
id: TASK-2869
title: 'API-14: missing_docs is enforced nowhere, so public-item doc gaps regress silently'
status: Done
assignee: []
created_date: '2026-11-18 21:42'
updated_date: '2026-11-19 08:08'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2882'
modified_files:
  - Cargo.toml
  - extensions/text-fixers/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `Cargo.toml` (`[workspace.lints.rust]`), `extensions/text-fixers/src/lib.rs`

**What**: No crate in the workspace runs `missing_docs`: TASK-2779 removed the only crate-root `#![warn(missing_docs)]` (config-checkers) to keep lint policy in `[workspace.lints]`, and declined adding it there because ~30 crates were never held to it. TASK-2827 asked for the same crate-root attribute in ops-text-fixers; it was not added for the same reason. ops-text-fixers and ops-config-checkers currently build clean under `-D missing_docs`, but nothing keeps them so.

**Why it matters**: The API-14 gaps fixed in TASK-2827 were only found by review; the next undocumented public item in these crates will be too.

**Origin**: discovered during TASK-2859 while fixing TASK-2827.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A workspace-level decision is recorded: missing_docs added to [workspace.lints.rust] at a gating level (with the undocumented items fixed or the opted-out crates listed), or a documented exemption mechanism lets doc-complete crates such as ops-text-fixers and ops-config-checkers opt in

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: missing_docs stays out of [workspace.lints] (about 30 crates were never held to it); doc-complete crates opt in through .ops.toml instead.
- A per-crate [lints] table cannot do it: Cargo rejects `[lints] workspace = true` plus extra lints ("cannot override workspace.lints in lints"), verified on a scratch workspace.
- Crate-root #![warn(missing_docs)] is ruled out by the policy that lint config lives in [workspace.lints].
- Chosen: leaf commands missing-docs-text-fixers / missing-docs-config-checkers running `cargo rustdoc -p <crate> --lib --all-features -- -D missing_docs` (the flag reaches that crate only, not its dependencies, and reuses the doc gate's build), grouped in a parallel `missing-docs` composite appended to `ops verify`. CI runs `ops verify`, so it is gated there too.
- Verified: an undocumented pub fn added to ops-text-fixers fails the gate; both crates pass as-is.
- Documented in docs/clippy.md under "missing_docs is opt-in per crate", with how to opt in another crate.
<!-- SECTION:NOTES:END -->
