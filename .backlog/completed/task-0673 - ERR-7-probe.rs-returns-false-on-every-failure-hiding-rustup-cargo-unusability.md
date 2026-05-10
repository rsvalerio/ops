---
id: TASK-0673
title: >-
  ERR-7: probe.rs returns false on every failure, hiding rustup/cargo
  unusability
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 17:56'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:42-47, 181-191`

**What**: `check_cargo_tool_installed` and `check_rustup_component_installed` map both spawn errors and non-success exits to `false` with no log. When `cargo --list` / `rustup component list` fails (rustup missing, permission denied, transient EAGAIN), every tool is reported `NotInstalled` and the install path then re-runs unconditionally; operators get no signal.

**Why it matters**: Compare to the tracing::warn discipline applied elsewhere (e.g. find_workspace_root, query_or_warn).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log spawn/failure with tracing::warn! including stderr tail before returning false
- [ ] #2 Distinguish 'tool absent' from 'rustup unusable' so callers can short-circuit
<!-- AC:END -->
