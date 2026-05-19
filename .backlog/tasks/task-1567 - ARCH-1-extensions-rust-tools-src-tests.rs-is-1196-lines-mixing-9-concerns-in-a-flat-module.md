---
id: TASK-1567
title: >-
  ARCH-1: extensions-rust/tools/src/tests.rs is 1196 lines mixing 9+ concerns in
  a flat module
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-19 16:10'
updated_date: '2026-05-19 16:46'
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
- [ ] #1 tools/src/tests.rs is split so no file exceeds ~400 lines
- [ ] #2 Each test group is co-located with the production module it covers (probe/cargo.rs, probe/rustup.rs, probe/path.rs, install.rs)
- [ ] #3 All tests continue to pass under cargo test -p ops-tools
<!-- AC:END -->
