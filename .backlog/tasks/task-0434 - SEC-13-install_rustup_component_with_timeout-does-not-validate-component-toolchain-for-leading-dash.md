---
id: TASK-0434
title: >-
  SEC-13: install_rustup_component_with_timeout does not validate
  component/toolchain for leading dash
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 04:43'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:75-90`

**What**: install_cargo_tool_with_timeout calls validate_cargo_tool_arg to reject crate names that begin with `-` (so they cannot be parsed as a flag by cargo). The sibling install_rustup_component_with_timeout passes both component and toolchain straight through to Command::args(["component", "add", component, "--toolchain", toolchain]) with no equivalent guard. A value like `--default` or `-vV` would be re-parsed by rustup as a flag.

**Why it matters**: The defense-in-depth rationale documented for validate_cargo_tool_arg (lines 15-21) applies identically here: Command::args blocks shell expansion but does not stop a leading `-` from being interpreted as a flag by the called binary. Same SEC-13 class TASK-0373 closed for cargo install.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Apply validate_cargo_tool_arg (or an equivalent shape check) to both component and toolchain before spawning rustup
- [ ] #2 Add a unit test asserting install_rustup_component_with_timeout("--default", "stable", _) is rejected before the subprocess spawns
<!-- AC:END -->
