---
id: TASK-0053
title: 'CD-11: Stack parallel match arms in manifest_files and default_commands_toml'
status: Done
assignee: []
created_date: '2026-04-14 20:32'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-2
  - DUP-5
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/stack.rs:34-121`
**Anchor**: `fn manifest_files`, `fn default_commands_toml`
**Impact**: Both methods contain 8+1 identical match arms over all Stack variants. Adding a new stack requires updating both methods in lockstep. A data-driven approach (e.g., a const array of `StackDef { manifests: &[&str], toml: Option<&str> }` indexed by variant) or a single `stack_metadata()` method returning both would consolidate the parallel match blocks.

DUP-2: 2 functions with parallel structure over the same enum. DUP-5: extract into shared data table.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Stack metadata defined in a single location; manifest_files and default_commands_toml derive from it
<!-- AC:END -->
