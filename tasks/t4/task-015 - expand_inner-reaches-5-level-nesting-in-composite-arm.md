---
id: TASK-015
title: expand_inner reaches 5-level nesting in composite arm
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-quality
  - CQ
  - FN-2
  - low
  - effort-S
  - crate-runner
dependencies: []
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/command/mod.rs:206-236`
**Anchor**: `fn expand_inner`
**Impact**: The Composite match arm (line 225) contains a cycle-detection guard (`if !visited.insert`) and a for-loop with recursive extend, reaching 5 levels (fn → match → arm → if/for → extend). The function also has 5 parameters (at the FN-3 threshold).

**Notes**:
This is a recursive tree-walk with cycle detection — the nesting is partially inherent. A simple improvement: use early-return for the Exec arm (`CommandSpec::Exec(_) => return Some(vec![id.to_string()])`) so the Composite arm doesn't need to be inside a match block, or extract the composite expansion body into a small helper. The 5 parameters are justified by the recursion signature.
<!-- SECTION:DESCRIPTION:END -->
