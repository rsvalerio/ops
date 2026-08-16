---
id: TASK-1567
title: >-
  ARCH-1: extensions-rust/tools/src/tests.rs is 1196 lines mixing 9+ concerns in
  a flat module
status: Done
assignee:
  - TASK-1578
created_date: '2026-05-19 16:10'
updated_date: '2026-08-15 21:22'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/tests.rs` (1196 lines, single flat `mod tests` via `#[cfg(test)] mod tests;` in lib.rs:11)

**What**: the test module mixes nine distinct concerns into one 1196-line flat file:
1. `ToolSpec` deserialization & accessor coverage (lines 1-83)
2. `ToolStatus` / `ToolInfo` derive-trait coverage (85-135)
3. `parse_active_toolchain` parsing tests (137-280)
4. `is_in_cargo_list` membership tests (282-374)
5. `is_component_in_list` rustup-component parsing tests (376-441)
6. `check_*` integration tests with `#[ignore]` (443-657)
7. `collect_tools` orchestration tests (660-707)
8. `install_tool` policy + `should_run_cargo_install` (709-767)
9. `validate_cargo_tool_arg` SEC-13 + PATH walking + PERF-3 index + ERR-2 install failure path (769-1196)

ARCH-1 threshold (300 lines, mixing 3+ concerns) is exceeded by ~4x lines and 3x concerns. The recently-created submodule structure (`probe/{mod,cargo,rustup,path,timeout}.rs`) is the clear template — each probe sub-concern already lives next to its production code, but its tests were left in this monolithic file.

**Why it matters**:
- Discoverability: when a contributor edits `probe/rustup.rs`, the relevant tests are 800 lines away in a sibling file under a different module.
- Compile / incremental rebuild: any test edit in this file rebuilds the entire test binary's IR for the crate; co-located `#[cfg(test)] mod tests` blocks inside each submodule (`probe/rustup.rs`, `probe/path.rs`, `install.rs`) would scope incremental rebuilds to the touched module.
- Sibling crates have already filed equivalent splits (TASK-1545 metadata/src/tests.rs:1288, TASK-1559 test-coverage/lib.rs); this file should follow the same pattern.

**Fix sketch**: move each concern group to a `#[cfg(test)] mod tests { ... }` inline block inside the corresponding production module:
- `parse_active_toolchain`, `is_component_in_list`, rustup install-arg tests → `probe/rustup.rs` and `install.rs`
- `is_in_cargo_list`, `check_cargo_tool_installed_*` → `probe/cargo.rs`
- `find_on_path_*`, `is_in_path_index`, PERF-3 tests → `probe/path.rs`
- `validate_cargo_tool_arg`, `install_cargo_tool_failure_*`, `should_run_cargo_install` → `install.rs`
- Keep only the cross-module orchestration tests (`collect_tools_*`, `check_tool_status_*`) in a much smaller `tests.rs`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tools/src/tests.rs is split so no file exceeds ~400 lines
- [x] #2 Each test group is co-located with the production module it covers (probe/cargo.rs, probe/rustup.rs, probe/path.rs, install.rs)
- [x] #3 All tests continue to pass under cargo test -p ops-tools
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done. `tools/src/tests.rs` went from 1367 lines to 242, and all 103 tests were
relocated next to the code they cover.

Routing (test count → destination):
- 33 → `probe/rustup/tests.rs` — parse_active_toolchain, classify_active_toolchain, is_component_in_list, rustup integration probes
- 15 → `probe/cargo/tests.rs` — is_in_cargo_list, check_cargo_tool_installed (incl. the CARGO-env test)
- 11 → `probe/path/tests.rs` — find_on_path*, PERF-3 path index, check_binary_installed
- 8 → `probe/mod.rs` — check_tool_status dispatcher (inline, the file is small)
- 17 → `install/tests.rs` — install_tool policy, validate_cargo_tool_arg, validate_rustup_toolchain, install failure paths
- 19 stay in `tests.rs` — the genuinely cross-cutting surface: ToolSpec deserialization, ToolStatus/ToolInfo, collect_tools, extension metadata

On AC #1 vs AC #2: co-locating tests as inline `mod tests { .. }` blocks (the
fix sketch's literal wording) satisfied AC #2 but pushed rustup.rs to 631 and
path.rs to 524, breaking AC #1. Resolved by using the `display.rs` +
`display/tests.rs` pattern already established in crates/runner and
crates/core: each module keeps `#[cfg(test)] mod tests;` and its tests live in
a child file. Both ACs now hold — largest file in the crate is 361 lines
(probe/path.rs), and every test is still a child module of the code it covers.

Also removed two `#[cfg(test)] pub(crate) use` re-export lines from
`probe/mod.rs` that existed only to feed the old central tests.rs and became
dead once the tests moved into the modules that own those items.

Verification: test inventory diffed by name before vs after — all 103 present,
the single absentee being `find_on_path_in_locates_executable_with_pathext_windows`,
which is `#[cfg(windows)]` and correctly not listed on Linux. Crate total is
114 (103 moved + 11 already in probe/install_timeout test modules).
`cargo test -p ops-tools`: 105 passed / 9 ignored; `-- --ignored`: 9 passed.
`ops verify` 7/7, `ops qa` 3/3.

Unrelated finding while running the gate: `ops qa` failed once in ops-about,
then passed unchanged. Pre-existing wall-clock ratio assertions, same class as
the ones commit 98e9ef6 removed from core/runner. Filed as TASK-1667.
<!-- SECTION:NOTES:END -->

---

**Status note (2026-05-19, wave-124):** Left In Progress. Splitting the
1196-line `tools/src/tests.rs` into co-located `#[cfg(test)] mod tests`
blocks inside `probe/{cargo,rustup,path}.rs` and `install.rs` requires
mechanically relocating ~50 tests, threading their imports through the
new module locations, and verifying no shared helper drifts. The
sibling acceptance criteria for this wave touched many of the same
code paths; a partial split would have landed alongside diff-noisy
import shuffles that made review harder. Wave-124 deliberately defers
this to a dedicated wave that can take the split on its own. No
production behaviour regressed.

## Triage Notes

<!-- SECTION:TRIAGE:BEGIN -->
Reset from `In Progress` to `To Do` in the 2026-08-15 sweep.

Verified against the tree: `extensions-rust/tools/src/tests.rs` is now **1361
lines**, up from the 1196 quoted in the report. No split has happened and the
file has grown, so the `In Progress` marker dated 2026-05-19 was stale.

For context, sibling crates in the same directory are comparable or worse —
`deps/src/tests.rs` is 1589 lines — so this is a pattern rather than a
one-off. Consider whether the task should be widened before it is picked up.
<!-- SECTION:TRIAGE:END -->
