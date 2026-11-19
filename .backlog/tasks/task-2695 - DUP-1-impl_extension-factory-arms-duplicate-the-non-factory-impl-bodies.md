---
id: TASK-2695
title: 'DUP-1: impl_extension! factory arms duplicate the non-factory impl bodies'
status: To Do
assignee: []
created_date: '2026-11-18 18:28'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2844'
modified_files:
  - crates/extension/src/macros.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:crates/extension/src/macros.rs:impl_extension'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/macros.rs:313-353` vs `crates/extension/src/macros.rs:356-381`; `crates/extension/src/macros.rs:384-412` vs `crates/extension/src/macros.rs:415-435`; `crates/extension/src/macros.rs:284`

**What**: `impl_extension!` has four public arms. Each `factory:` arm repeats, token for token, the entire `impl $crate::Extension for $struct { ... }` block of its non-factory sibling (25+ identical lines for the full form, ~10 for the short form) and only appends the `distributed_slice` static. The `@accessors` internal rule also takes a `$struct:ty` fragment it never uses.

**Why it matters**: DUP-1. Any change to the generated impl (a new accessor, a changed `register_commands` default, the MACRO-3 linkme fix) has to be made in two places per form, and a miss produces extensions that behave differently depending only on whether they auto-register.

**Suggested fix**: have each factory arm expand to the corresponding non-factory arm via `$crate::impl_extension!(...)` and then emit only the static; drop the unused `$struct` parameter from `@accessors`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Extension impl body is written once per form; factory arms delegate to the non-factory arm and add only the registry static
- [ ] #2 The @accessors rule takes no unused fragments
- [ ] #3 All impl_extension! invocations in the workspace still compile and their tests pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.04 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
