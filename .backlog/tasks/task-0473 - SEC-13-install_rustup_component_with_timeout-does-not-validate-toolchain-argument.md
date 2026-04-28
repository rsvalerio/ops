---
id: TASK-0473
title: >-
  SEC-13: install_rustup_component_with_timeout does not validate toolchain
  argument
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 17:48'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:71-90`

**What**: install_cargo_tool_with_timeout validates name/package via validate_cargo_tool_arg (rejecting leading `-`), but the sibling install_rustup_component_with_timeout passes `component` and `toolchain` to Command::args with no validation. TASK-0434 targets `component` specifically; this finding extends to the `toolchain` parameter.

**Why it matters**: SEC-13 defense-in-depth — same shape as the existing cargo install validation. While Command::args avoids shell interpretation, a leading `-` value would still be parsed as a rustup flag.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 validate_cargo_tool_arg (or a rustup-specific equivalent) is applied to both component and toolchain in install_rustup_component_with_timeout
- [x] #2 Unit-test rejecting "-foo" for both parameters
<!-- AC:END -->
