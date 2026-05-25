---
id: TASK-1582
title: 'PATTERN-1: CARGO_BUILTIN_SUBCOMMANDS hardcoded list drifts as cargo evolves'
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-21 22:45'
updated_date: '2026-05-22 12:50'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/cargo.rs:13-53`

**What**: `is_in_cargo_list` filters cargo built-ins out of `cargo --list` output via a hand-curated `const CARGO_BUILTIN_SUBCOMMANDS` array. The list is hardcoded against a snapshot of cargo's surface (no `verify-project`-style version qualifier in sight) and will drift as cargo gains new built-ins. A new built-in (e.g. a future `cargo deps`, `cargo audit`) appearing in `cargo --list` will be misread as an installed cargo tool, causing `check_tool_status` to report `Installed` for a name the user wanted us to `cargo install` instead.

**Why it matters**: PATTERN-1 — when behavior depends on an external tool's evolving surface, encode the dependency by querying that tool (e.g. parse the prefix marker `cargo --list` uses to separate built-ins from installed binaries, or distinguish via the path returned by `cargo --list -v` / by checking whether the binary lives next to the cargo executable) rather than maintaining a manual mirror. At minimum, gate the list on a `rustc --version` test in CI so a built-in addition is caught before it ships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 is_in_cargo_list no longer relies on a hand-curated cargo built-in list, OR the list is paired with a CI check that fails when cargo gains new built-ins
- [x] #2 existing tools probe tests continue to pass on stable cargo
- [x] #3 decision and rationale recorded as a module-level comment
<!-- AC:END -->
