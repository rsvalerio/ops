---
id: TASK-1568
title: >-
  TEST-1: tool_status_debug and tool_status_clone pin compiler-generated derive
  output without behavioural value
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-19 16:10'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/tests.rs:94-105`

**What**: two tests assert exactly what `#[derive(Debug, Clone, Copy)]` generates:

```rust
#[test]
fn tool_status_debug() {
    assert_eq!(format!("{:?}", ToolStatus::Installed), "Installed");
    assert_eq!(format!("{:?}", ToolStatus::NotInstalled), "NotInstalled");
}

#[test]
fn tool_status_clone() {
    let status = ToolStatus::Installed;
    let cloned = status;
    assert_eq!(status, cloned);
}
```

`tool_status_debug` asserts that `#[derive(Debug)]` produces the variant name verbatim — that is the *definition* of `derive(Debug)`. `tool_status_clone` does an implicit `Copy` (not even a `Clone::clone` call) and asserts `PartialEq`.

These are tautological derive-coverage tests — they cannot fail without removing the derive itself, and removing the derive is already a compile-time break in many other places. The `ToolStatus` type also has an explicit `impl std::fmt::Display` (lib.rs:85-94) whose user-facing strings *are* the deliberate contract per the doc comment ("deliberate contract — not a Debug byproduct that mutates whenever a variant gains a field") — but no test exercises `Display`, so the actual contract is unpinned while the throw-away `Debug` shape is over-pinned.

**Why it matters**:
- TEST-1 / TEST-11: an assertion-shaped test that cannot drive a behavioural failure adds noise and discourages real coverage.
- Inverts the contract: lib.rs:50-72 documents `Display` as the stable user-facing surface and `Debug` as incidental, but the tests pin the opposite.
- The doc comment specifically warns: "**When adding a variant:** extend the `Display` impl below with an intentional, stable user-facing string before merging." — but a new variant will still pass `tool_status_debug` (the variant just won't appear in the assertion).

**Fix sketch**: delete `tool_status_debug` and `tool_status_clone`; replace with `tool_status_display_strings_are_stable` that asserts each variant's `Display` output matches the documented contract (`"installed"`, `"not installed"`, `"probe failed"`), so a future variant addition that forgets the `Display` arm fails CI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tool_status_debug and tool_status_clone are removed or replaced
- [ ] #2 A new test pins ToolStatus::Display output for every variant (Installed, NotInstalled, ProbeFailed)
- [ ] #3 Adding a new ToolStatus variant without extending the Display impl causes the test to fail to compile or assert
<!-- AC:END -->
