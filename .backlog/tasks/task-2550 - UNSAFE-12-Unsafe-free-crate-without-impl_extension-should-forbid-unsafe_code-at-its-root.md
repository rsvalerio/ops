---
id: TASK-2550
title: 'UNSAFE-12: Unsafe-free crate without impl_extension! should forbid unsafe_code at its root'
status: Done
assignee: []
created_date: '2026-10-10 15:39'
updated_date: '2026-10-10 22:04'
labels:
  - code-review
  - unsafe
dependencies: []
parent_task_id: 'TASK-2620'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'UNSAFE-12:extensions/create-review-tasks/src/lib.rs:<crate root>'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:1` (crate root)

**What**: The crate contains no `unsafe` and invokes no macro that expands to `unsafe` — unlike every other `extensions/*` member, which invokes `impl_extension!` — yet its root carries no `#![forbid(unsafe_code)]`. It inherits only the workspace's `unsafe_code = "deny"` via `[lints] workspace = true`. The workspace's own `[workspace.lints.rust]` comment in the root Cargo.toml states the convention explicitly: "`deny`, not "forbid": `impl_extension!` puts `#[allow(unsafe_code)]` on the linkme static it generates, which `forbid` rejects. Unsafe-free crates that do not invoke it add `#![forbid(unsafe_code)]` at their root." Comparable unsafe-free library crates already follow it: `crates/backlog`, `crates/extension`, `crates/theme`, and the `extensions-rust/foundation/templates/lints.toml` template for new extensions.

**Why it matters**: UNSAFE-12: a crate that does not need `unsafe` should say so mechanically. `forbid` cannot be lifted by a later `#[allow(unsafe_code)]`, so the property survives the well-meaning PR that adds one `unsafe` block, and "does this crate contain unsafe?" becomes a fact the build enforces rather than a question a reviewer re-answers. This crate is exactly the case the workspace carve-out describes: no `impl_extension!`, so nothing blocks `forbid`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 #![forbid(unsafe_code)] is added at the crate root of src/lib.rs
- [x] #2 cargo check -p ops-create-review-tasks and cargo test -p ops-create-review-tasks still pass

<!-- AC:END -->
