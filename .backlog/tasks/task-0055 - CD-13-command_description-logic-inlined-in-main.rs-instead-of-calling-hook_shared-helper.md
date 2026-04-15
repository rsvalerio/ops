---
id: TASK-0055
title: >-
  CD-13: command_description logic inlined in main.rs instead of calling
  hook_shared helper
status: Done
assignee: []
created_date: '2026-04-14 20:32'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-1
  - DUP-5
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/main.rs` (`print_categorized_help`), `crates/cli/src/hook_shared.rs:128-132`
**Anchor**: `fn command_description` (hook_shared), inline in `print_categorized_help`
**Impact**: `hook_shared::command_description` extracts `spec.help().unwrap_or_else(|| spec.display_cmd_fallback())` into a public helper. `print_categorized_help` in main.rs re-implements the same expression inline instead of calling the existing helper. If the fallback logic changes, only one site would be updated.

DUP-1: identical 3-line expression. DUP-5: call the existing helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 print_categorized_help calls hook_shared::command_description instead of inlining the expression
<!-- AC:END -->
