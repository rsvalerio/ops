---
id: TASK-2511
title: 'UNSAFE-12: ops-rust-foundation is unsafe-free but does not mechanically forbid unsafe code'
status: To Do
assignee: []
created_date: '2026-10-10 15:32'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - unsafe
dependencies: []
parent_task_id: 'TASK-2616'
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'UNSAFE-12:extensions-rust/foundation/src/lib.rs:lib'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs` (crate root)

**What**: The crate contains no `unsafe`, but its root carries no `#![forbid(unsafe_code)]` and the centralized lint policy (`[workspace.lints.rust]` in the root `Cargo.toml`) sets `unsafe_code = "deny"`, not `forbid`. A later module- or item-level `#[allow(unsafe_code)]` can therefore lift the ban silently. The workspace's own lint-policy comment states the convention: unsafe-free crates that do not invoke `impl_extension!` add `#![forbid(unsafe_code)]` at their root. This crate has no `linkme`/`impl_extension!` usage (no such dependency in `extensions-rust/foundation/Cargo.toml`), so `forbid` is applicable with no macro-expansion caveat.

**Why it matters**: `forbid` cannot be lifted by a well-meaning PR that adds one `unsafe` block for a micro-optimization, so "this crate contains no unsafe" becomes a fact the build enforces rather than a question every reviewer re-answers (UNSAFE-12).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 #![forbid(unsafe_code)] added at the crate root of extensions-rust/foundation/src/lib.rs
- [ ] #2 cargo check -p ops-rust-foundation and cargo test -p ops-rust-foundation pass unchanged
<!-- AC:END -->
