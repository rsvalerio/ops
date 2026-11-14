---
id: TASK-2363
title: 'UNSAFE-12: ops-about-go is unsafe-free but does not deny unsafe_code mechanically'
status: Done
assignee: []
created_date: '2026-10-04 14:10'
updated_date: '2026-10-04 14:31'
labels:
  - code-review-rust
  - unsafe
dependencies: []
modified_files:
  - extensions-go/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'UNSAFE-12:extensions-go/about/src/lib.rs:crate'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:12`

**What**: The crate holds no hand-written `unsafe` (go_mod, go_syntax, go_work, modules, lib are all safe code) but its root carries no `unsafe_code` lint, and `[workspace.lints]` in the root `Cargo.toml` has none either. Sibling crates with no linkme use carry `#![forbid(unsafe_code)]`; this crate cannot use `forbid` because `impl_extension!` (via `linkme::distributed_slice`) expands to `#[link_section]` statics whose unsafe tokens count against the invoking crate. `crates/extension/src/macros.rs` already emits `#[allow(unsafe_code)]` on the generated static for invoking crates that only `deny`, so `#![deny(unsafe_code)]` is the supported level (UNSAFE-12 names `deny` plus scoped `#[expect]` for exactly this case).

**Why it matters**: A later PR can add an `unsafe` block to this parser crate with no build signal; "does this crate contain unsafe?" stays a question each reviewer re-answers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 lib.rs carries #![deny(unsafe_code)] (with a comment explaining why not forbid, per the ops-extension note), or the workspace lints table sets unsafe_code
- [ ] #2 cargo clippy -p ops-about-go --all-targets passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Superseded by TASK-2368 (workspace-wide unsafe_code lint). Closed as a duplicate; no code change.
<!-- SECTION:NOTES:END -->
