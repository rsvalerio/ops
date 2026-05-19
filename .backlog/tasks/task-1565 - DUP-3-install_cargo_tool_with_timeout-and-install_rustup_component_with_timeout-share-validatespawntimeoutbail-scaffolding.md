---
id: TASK-1565
title: >-
  DUP-3: install_cargo_tool_with_timeout and
  install_rustup_component_with_timeout share validate+spawn+timeout+bail
  scaffolding
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-19 15:56'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:62`, `extensions-rust/tools/src/install.rs:119`

**What**: Both functions validate args via `validate_cargo_tool_arg`, build a `Command::new(resolve_*_bin())` with `.args(...).stdin(Stdio::null()).stdout(inherit()).stderr(inherit()).spawn().context("failed to spawn ...")?`, call `run_with_timeout(child, timeout, &format!(...))?`, and branch on `status.success()` with `anyhow::bail!`. ~15 lines duplicated per side.

**Why it matters**: The shared scaffold encodes a policy contract (stdin closed, stdout/stderr inherited, timeout-bounded, non-zero -> bail) that should live in one place. New install paths (e.g. a future `cargo binstall` or `rustup toolchain install`) will copy-paste the same shape, and policy changes must be repeated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a spawn_install_with_timeout(bin, args, timeout, label) -> anyhow::Result<ExitStatus> helper (or similar) encapsulating the stdio policy and timeout wait
- [ ] #2 Both install_*_with_timeout call the helper; per-site logic reduces to validation + args construction + the package/name-aware failure message
- [ ] #3 Existing failure-message tests (install_cargo_tool_failure_names_both_package_and_bin, _without_package_keeps_single_identifier, install_rustup_component_rejects_dash_*) stay green
<!-- AC:END -->
