---
id: TASK-1592
title: 'READ-7: ProbeOutcome::map is dead code, kept alive with #[allow(dead_code)]'
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-21 22:49'
updated_date: '2026-05-22 13:18'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/timeout.rs:30-38`

**What**: `ProbeOutcome::<T>::map` is declared `pub(crate)` and decorated with `#[allow(dead_code)]`. Grepping the crate (and the wider workspace) shows zero call sites — the four production paths (`run_probe_capturing`, `run_probe_with_timeout`, `check_cargo_tool_installed`, `check_rustup_component_installed`) all match on `ProbeOutcome::{Ok, Failed}` explicitly. The `#[allow]` was added so the helper would compile, but nothing depends on it.

```rust
impl<T> ProbeOutcome<T> {
    #[allow(dead_code)]
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> ProbeOutcome<U> { ... }
}
```

**Why it matters**: READ-7 / dead-code maintenance burden. A `#[allow(dead_code)]` annotation on a `pub(crate)` API masks the signal that this helper has no purpose. Either:
- delete the impl (and the `#[allow]`), or
- adopt it at one of the existing `ProbeOutcome::Ok(t) => ProbeOutcome::Ok(f(t))` sites in `cargo.rs:62-67` / `rustup.rs:81-85` so the combinator earns its keep.

A reviewer adding a new probe in six months will see the `#[allow(dead_code)]` and assume the helper is part of the supported surface; deleting unused code is preferable to permanently silencing the lint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ProbeOutcome::map is either removed (and #[allow(dead_code)] with it) or wired into at least one call site so the dead-code allow is unnecessary
- [x] #2 no remaining #[allow(dead_code)] annotations on ProbeOutcome's inherent impl
<!-- AC:END -->
