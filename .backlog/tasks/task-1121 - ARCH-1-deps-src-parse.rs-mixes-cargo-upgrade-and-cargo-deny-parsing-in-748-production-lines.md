---
id: TASK-1121
title: >-
  ARCH-1: deps/src/parse.rs mixes cargo-upgrade and cargo-deny parsing in 748
  production lines
status: Done
assignee:
  - TASK-1264
created_date: '2026-05-08 07:27'
updated_date: '2026-05-09 12:08'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:1-748`

**What**: A single source file owns two unrelated parsers — the `cargo upgrade --dry-run` table parser (lines ~19-386) and the `cargo deny check` JSON parser (lines ~388-748). Each has its own types (`UpgradeParseDiagnostics`, `DiagClass`, `DenyLine`, `DenyAdvisory`), constants (`CARGO_UPGRADE_TIMEOUT`, `CARGO_DENY_TIMEOUT`, `CODE_CLASSES`, `MISSING_SEVERITY_SENTINEL`), and entry points (`run_cargo_upgrade_dry_run` / `interpret_upgrade_output`, `run_cargo_deny` / `interpret_deny_result`). Total file is 748 lines of production code (no `#[cfg(test)]`).

**Why it matters**: ARCH-1 flags >500-line modules that mix unrelated concerns. The two pipelines share no data shapes — one parses a fixed-column ASCII table from stdout, the other parses NDJSON diagnostics from stderr. A change to either parser (cargo-edit format drift, cargo-deny schema bump) currently churns the same file and forces reviewers to context-switch between two unrelated state machines. Splitting into `parse_upgrade.rs` + `parse_deny.rs` (or sibling submodules under `parse/`) would let each parser carry its own constants, types, and tests in isolation, and would make adding a third tool integration (e.g. `cargo audit`) a new file rather than another section in an already-overweight one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Split parse.rs into per-tool modules (e.g. parse/upgrade.rs and parse/deny.rs) keeping the existing public API surface
- [x] #2 Each new module is under ~400 lines and owns its own constants, types, and helpers
- [x] #3 tests/parse_upgrade.rs and tests/parse_deny.rs (or equivalent) move alongside their parser
<!-- AC:END -->
