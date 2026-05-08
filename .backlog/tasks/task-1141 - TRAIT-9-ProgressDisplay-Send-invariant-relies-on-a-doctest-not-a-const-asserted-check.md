---
id: TASK-1141
title: >-
  TRAIT-9: ProgressDisplay !Send invariant relies on a doctest, not a
  const-asserted check
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 07:41'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - TRAIT
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:87-103`

**What**: The CL-3/TASK-0656 invariant (\"handle_event must never be polled from a multi-thread tokio worker\") is encoded as `_not_send: PhantomData<*const ()>` with a `compile_fail` doctest. The doctest is a doc-string check, not CI-enforced — only fails if rustdoc actually runs that snippet. The `static_assert_not_send` block lives in `display/tests.rs` (gated `#[cfg(test)]`), so a build without tests does not exercise it.

**Why it matters**: A field renamed or replaced (e.g. by a derive proc-macro that adds a Send field) compiles cleanly in production and only fails under cargo test. The invariant is load-bearing for CONC-5 — silently regressing it lets handle_event land on a multi-thread tokio worker and re-introduce the synchronous-stderr-write hang.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move assert_not_send check into a const _: () block at module top level (compiled in every build)
- [ ] #2 Or place it in a public-doctest that runs under cargo test --doc and compile_fail
<!-- AC:END -->
