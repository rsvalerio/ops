---
id: TASK-2575
title: 'UNSAFE-12: ops-hook-common is unsafe-free but does not mechanically forbid unsafe code'
status: To Do
assignee: []
created_date: '2026-10-10 15:43'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - unsafe
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'UNSAFE-12:extensions/hook-common/src/lib.rs:lib'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:1`

**What**: The crate contains no `unsafe` and does not invoke `impl_extension!` (verified by grep), yet its root lacks `#![forbid(unsafe_code)]`. The workspace policy in the root Cargo.toml [workspace.lints.rust] sets `unsafe_code = "deny"` (not forbid, because impl_extension! needs a scoped allow) and states explicitly: 'Unsafe-free crates that do not invoke it add #![forbid(unsafe_code)] at their root'. ops-hook-common is one of the crates that policy point at, and it does not.

**Why it matters**: UNSAFE-12 — `forbid` cannot be lifted by a later `#[allow(unsafe_code)]` on a module or function, so the unsafe-free property survives a well-meaning PR and becomes a build-enforced fact instead of a question each reviewer re-answers. Sibling finding: TASK-2511 (ops-rust-foundation, same rule).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 lib.rs carries #![forbid(unsafe_code)] at the crate root
- [ ] #2 cargo check and cargo test for ops-hook-common pass with the forbid in place
<!-- AC:END -->
