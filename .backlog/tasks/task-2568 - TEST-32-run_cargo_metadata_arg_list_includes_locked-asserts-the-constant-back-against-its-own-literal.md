---
id: TASK-2568
title: 'TEST-32: run_cargo_metadata_arg_list_includes_locked asserts the constant back against its own literal'
status: Done
assignee: []
created_date: '2026-10-10 15:42'
updated_date: '2026-10-10 21:49'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2619'
modified_files:
  - extensions-rust/metadata/src/tests/wiring.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-rust/metadata/src/tests/wiring.rs:run_cargo_metadata_arg_list_includes_locked'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests/wiring.rs:199-205`

**What**: The test asserts `CARGO_METADATA_ARGS == ["metadata", "--format-version", "1", "--locked"]` — the exact literal the const is defined with in `src/lib.rs:237`. The assertion passes by construction: it can only fail when someone edits the const, at which point the diff already says what changed, and it does not verify any behavior of `run_cargo_metadata` (a mutant that stops consulting the const survives, as the test's own doc concedes — it leans on the `-D warnings` dead-code gate instead).

**Why it matters**: TEST-32 — a test must not assert ground truth back to itself; the constant-equals-its-own-literal shape still has to be read and re-approved in every future diff while never failing for a reason anyone cares about. The named invariant (--locked must reach cargo) is better pinned as a property the value must satisfy, e.g. `assert!(CARGO_METADATA_ARGS.contains(&"--locked"))` plus a first-element check, or by asserting on the arguments a spawned/stubbed `run_cargo` receives. Note the sibling `ceiling_is_exactly_u32_max` in `src/tests/payload_cap.rs:257` is the good shape already: it checks the const against an independently expressed bound (`u64::from(u32::MAX)`), not its own literal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test asserts a property of the argument list (contains --locked; starts with the metadata subcommand) or the arguments actually handed to run_cargo, instead of restating the const's literal

<!-- AC:END -->
