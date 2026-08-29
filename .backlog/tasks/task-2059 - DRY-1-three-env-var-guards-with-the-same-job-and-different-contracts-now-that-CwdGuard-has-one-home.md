---
id: TASK-2059
title: >-
  DRY-1: three env-var guards with the same job and different contracts, now
  that CwdGuard has one home
status: Done
assignee:
  - TASK-2061
created_date: '2026-08-29 13:54'
updated_date: '2026-08-29 18:28'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - crates/cli/src/test_utils.rs
  - extensions/hook-common/src/test_helpers.rs
  - extensions-rust/deps/src/test_support.rs
  - crates/core/src/test_utils.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/test_utils.rs`, `extensions/hook-common/src/test_helpers.rs`, `extensions-rust/deps/src/test_support.rs`

**What**: TASK-2045 collapsed the workspace's three `CwdGuard` implementations
into one, in `ops_core::test_utils`. Their sibling env-var guards were left
untouched and are the same DRY-1 shape:

- `ops_cli::test_utils::EnvVarGuard` — `unset` / `set` / `set_value` /
  `unset_value`, `&'static str` key, `OsString` values.
- `ops_hook_common::test_helpers::EnvGuard` — `remove` / `set`, `String`
  values, different name for the same concept.
- `ops_deps::test_support::EnvVarGuard` — a third copy, `OsString`-keyed.

`ops_core::test_utils` already exports its own `EnvGuard` as well, so the
workspace carries four.

**Why it matters**: DRY-1 — one concept, four implementations, two spellings of
the name, and differing value types, all of which restore process-global state
on drop and all of which depend on the caller remembering
`#[serial_test::serial]`. That is the exact argument TASK-2034 made about
`CwdGuard`: a new test reaches for whichever copy its crate happens to expose,
and the copies drift. Unlike `CwdGuard` there is no mutex to hoist, so the fix
is a straight consolidation onto `ops_core::test_utils` with re-exports, which
is now a well-worn path in this tree.

**Origin**: discovered during TASK-2045 while fixing TASK-2034.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One env-var guard exists in the workspace, in ops_core::test_utils, re-exported by the crates that used to define their own
- [x] #2 The surviving guard covers every constructor the four copies offered (set, unset/remove, and OsStr-valued keys) so no call site loses a capability
- [x] #3 Existing import paths keep working via re-export, or every call site is updated; the suites stay green under cargo test and nextest
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-2061: ops_core::test_utils::EnvGuard is now the workspaces only env-var guard. It is OsStr-keyed and OsStr-valued and carries every constructor the four copies offered: set, remove, unset (alias), set_value, unset_value. ops_cli::test_utils re-exports it as EnvVarGuard, ops_deps::test_support re-exports it as EnvVarGuard, ops_hook_common::test_helpers re-exports it as EnvGuard, so every existing import path and call site is unchanged. Two call sites lost a now-needless & on a format! argument (clippy::needless_borrows_for_generic_args) after the value type widened to AsRef<OsStr>. Suites green: cargo nextest run --workspace --all-features 2966 passed, cargo test --workspace --all-features --doc clean.
<!-- SECTION:NOTES:END -->
