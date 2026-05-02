---
id: TASK-0914
title: 'ASYNC-6: Rust tools probe spawns cargo/rustup without timeout'
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 12:12'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:9`

**What**: get_active_toolchain (line 9), capture_cargo_list (line 342), capture_rustup_components (line 354), check_cargo_tool_installed (line 52), and check_rustup_component_installed (line 206) all spawn cargo/rustup via Command::output() with no timeout. A wedged registry probe, broken sccache shim, or rustup proxy can hang `ops about` / `ops tools list` indefinitely.

**Why it matters**: TASK-0791 closed the same hazard for the --version probes, but the listing probes (called per About invocation and per `ops tools list`) still inherit the wedge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All five probe spawns route through run_with_timeout (or a sibling helper) honoring OPS_SUBPROCESS_TIMEOUT_SECS
- [ ] #2 Timeout maps to a tracing::warn and ToolStatus::Unknown / None rather than a hang
<!-- AC:END -->
