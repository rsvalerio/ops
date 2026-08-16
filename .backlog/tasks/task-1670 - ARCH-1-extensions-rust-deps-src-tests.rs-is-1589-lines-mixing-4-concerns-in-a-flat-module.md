---
id: TASK-1670
title: >-
  ARCH-1: extensions-rust/deps/src/tests.rs is 1589 lines mixing 4 concerns in a
  flat module
status: Triage
assignee: []
created_date: '2026-08-16 09:52'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/tests.rs` (1589 lines, 82 tests, flat `mod tests` declared from `lib.rs:21`)

Now the largest test file in the repo — larger than `tools/src/tests.rs` ever was (1367 at its peak). TASK-1567's triage note flagged this file and asked whether that task should be widened to cover it; the answer was no, `tools` was split on its own. This is the follow-through.

**What**: four concerns in one file, each already having a production module that owns it. The crate is well split (`lib.rs`, `format.rs`, `parse/deny.rs`, `parse/upgrade.rs`) — only the tests were left behind:

| Tests | Concern | Belongs in |
|---|---|---|
| 32 | `parse_deny_*`, `interpret_deny_result_*` | `parse/deny.rs` |
| 22 | `parse_upgrade_table_*`, `categorize_upgrades_*`, `interpret_upgrade_output_*` | `parse/upgrade.rs` |
| 17 | `has_issues_*` (13), `check_tool_in_*`, `schema_*`, `deps_report_serialization_*`, `provide_*` | `lib.rs` |
| 12 | `format_report_*` + its `render` helper | `format.rs` |

**Why it matters**: ARCH-1's threshold is 300 lines / 3+ concerns; this is 5x the lines. A contributor editing `parse/deny.rs` finds its tests in a sibling file under a different module, ~800 lines from the tests for `parse/upgrade.rs`. Editing any test rebuilds the whole crate's test IR.

## Follow the pattern established by TASK-1567

`tools` was split this way and both ACs held. Do not repeat the intermediate step that failed there: co-locating as **inline** `#[cfg(test)] mod tests { .. }` blocks satisfies co-location but pushes the production files well past 400 lines. Use the `module.rs` + `module/tests.rs` layout already used by `crates/runner/src/display.rs`, `crates/core/src/config/` and now `extensions-rust/tools/src/probe/`:

- `parse/deny.rs` keeps `#[cfg(test)] mod tests;`, tests move to `parse/deny/tests.rs`
- same for `parse/upgrade.rs` and `format.rs`
- `lib.rs`-owned tests can stay in a much smaller `tests.rs`

Note `parse/upgrade.rs` already has an inline `mod tests` (line 345) and `format.rs` has `mod helper_tests` (line 351). Merge into them or keep them alongside — either is fine, but do not end up with two modules named `tests` in one file.

## Constraint: do not re-duplicate the tracing scaffold

`tests.rs:5` defines a shared `mod tracing_capture` (the `BufWriter` + `MakeWriter` scaffold) with call sites at lines 201, 378 and 1465 — which land in **three different destination modules** under this split. TASK-1494 (Done) consolidated that scaffold specifically to remove the duplication; a split that gives each module its own copy silently reverts it. Give it one home reachable from all three, e.g. a `#[cfg(test)] pub(crate) mod` in `lib.rs` or a `test_support` module.

Two smaller inline modules, `extension_tests` (line 50) and `user_config_tests` (line 62), are `lib.rs`-level and can stay put.

## Scope

This task is `deps` only. The same pattern remains in other crates and is deliberately not bundled here — file separately if picked up:

| File | Lines |
|---|---|
| `crates/cli/src/run_cmd/tests.rs` | 1109 |
| `extensions-rust/test-coverage/src/tests.rs` | 886 |
| `extensions-rust/cargo-update/src/tests.rs` | 833 |
| `crates/extension/src/tests.rs` | 800 |
| `crates/cli/src/registry/tests.rs` | 786 |

## Verifying nothing is lost

The mechanical risk is dropping a test during the move. Diff the inventory by name before and after — `cargo test -p ops-deps -- --list --include-ignored` — rather than trusting the pass count, and account for any `#[cfg(...)]`-gated test that legitimately does not appear on this platform.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No file in extensions-rust/deps/src exceeds ~400 lines
- [ ] #2 Each test group is co-located with the production module it covers (parse/deny.rs, parse/upgrade.rs, format.rs, lib.rs) via module/tests.rs, not inline blocks that re-inflate the production files
- [ ] #3 The tracing_capture scaffold has exactly one definition, shared by all call sites
- [ ] #4 Test inventory diffed by name before and after: no test lost, any absentee explained by a cfg gate
- [ ] #5 cargo test -p ops-deps passes, and ops verify and ops qa pass
<!-- AC:END -->
