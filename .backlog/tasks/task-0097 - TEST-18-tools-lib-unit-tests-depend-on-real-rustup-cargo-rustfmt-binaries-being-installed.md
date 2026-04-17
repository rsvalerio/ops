---
id: TASK-0097
title: >-
  TEST-18: tools lib unit tests depend on real rustup/cargo/rustfmt binaries
  being installed
status: Done
assignee: []
created_date: '2026-04-17 11:56'
updated_date: '2026-04-17 16:17'
labels:
  - rust-code-review
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs` (tests `check_binary_installed_finds_rustup`, `check_cargo_tool_installed_fmt`, `check_rustup_component_installed_rustfmt`, `check_tool_status_simple_installed`, `check_tool_status_extended_with_rustup_component`, `check_tool_status_system_binary`, `get_active_toolchain_returns_some`, lines ~499-604)

**What**: These unit tests shell out to real `cargo`/`rustup` binaries via `Command::new(...).output()`. Their pass/fail depends on system state: `rustup` being installed, `rustfmt` component being present, and the active toolchain resolving. They pass on the developer machine (where `cargo install` works) and fail on minimal CI images, devcontainers, or cross-compile jobs that don't have the full rustup toolchain.

**Why it matters**: Violates TEST-18 (isolated state per test). Silent coverage loss on leaner environments, false red/green signal depending on where `cargo test` runs, and makes the crate hostile to contributors who install `rustc` via distro packages. The non-deterministic pass/fail is a classic flakiness pattern (see flakiness-patterns.md: "tests depend on external state").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Subprocess invocations (rustup component list, cargo --list, rustup show active-toolchain) are extracted behind a small trait or function-pointer so tests can inject fake stdout
- [x] #2 Existing parser-level tests (is_in_cargo_list, parse_active_toolchain, is_component_in_list) remain as pure-string tests without subprocess calls
- [x] #3 Tests that genuinely need rustup/rustfmt are either removed (covered by the parser tests + doctests) or marked #[ignore = "requires rustup + rustfmt installed, run with: cargo test -- --ignored"] with a documented reason
- [x] #4 cargo test passes on an environment with rustup uninstalled
<!-- AC:END -->
