---
id: TASK-2457
title: 'UNSAFE-12: ops-tfplan is unsafe-free but does not mechanically forbid unsafe code'
status: Done
assignee: []
created_date: '2026-10-10 15:25'
updated_date: '2026-10-10 21:53'
labels:
  - code-review-rust
  - unsafe
dependencies: []
parent_task_id: 'TASK-2622'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'UNSAFE-12:extensions-terraform/plan/src/lib.rs:lib'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:1`

**What**: `ops-tfplan` contains zero `unsafe` code (verified across `src/lib.rs`, `src/model.rs`, `src/describe.rs`, `src/render.rs` — the only unsafe-adjacent items are `libc::O_NOFOLLOW`/`ELOOP` constants and the `wait-timeout` crate, whose unsafe lives in the dependency's own compilation unit), but neither the crate root nor its lints say so mechanically. The crate inherits `[workspace.lints.rust] unsafe_code = "deny"` (root `Cargo.toml`), and `deny` — unlike `forbid` — can be lifted by a later `#[allow(unsafe_code)]` on a module or fn. The workspace lint table's own comment says unsafe-free crates that do not invoke `impl_extension!` add `#![forbid(unsafe_code)]` at their root; `ops-tfplan` invokes no such macro (no linkme/extension registry) and has no root attribute.

**Why it matters**: UNSAFE-12 — `forbid` makes "this crate contains no unsafe" a build-enforced fact instead of a property every reviewer re-derives, and it survives the well-meaning PR that adds "just one unsafe block" for a micro-optimization. This crate parses untrusted terraform plan JSON and hardens secret-bearing artifacts; it is exactly where a future unsafe block should be a compile error. None of the crate's dependencies expand unsafe tokens into this compilation unit, so `#![forbid(unsafe_code)]` at the root builds cleanly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An `unsafe_code = "forbid"` policy is mechanically reachable for ops-tfplan: `#![forbid(unsafe_code)]` at the crate root of extensions-terraform/plan/src/lib.rs (the per-crate route the workspace lint policy documents), and it is not lifted by any local allow
- [x] #2 cargo check -p ops-tfplan and cargo test -p ops-tfplan still pass with the attribute in place

<!-- AC:END -->
