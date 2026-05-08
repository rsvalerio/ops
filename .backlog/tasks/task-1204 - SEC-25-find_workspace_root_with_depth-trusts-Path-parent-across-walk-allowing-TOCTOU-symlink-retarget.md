---
id: TASK-1204
title: >-
  SEC-25: find_workspace_root_with_depth trusts Path::parent() across walk,
  allowing TOCTOU symlink retarget
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 08:16'
updated_date: '2026-05-08 14:13'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:343-435`

**What**: The walk reaches each ancestor via current.parent() on the lexical path of the canonicalized start, never re-canonicalising at each step, and manifest_declares_workspace reads each candidate by its lexical path. An attacker who can write inside any reachable ancestor can plant a Cargo.toml containing [workspace] and the walk will return that ancestor as the root — every downstream provider (units, coverage, deps) then targets the wrong workspace.

**Why it matters**: The doc-comment acknowledges the attack but offers no defensive option. At minimum the function should expose a find_workspace_root_strict variant that re-canonicalises each candidate manifest's parent dir before accepting it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A find_workspace_root_strict variant exists that re-canonicalises each candidate Cargo.toml's parent before returning it as the root, and rejects (with tracing::warn) candidates whose canonical path would escape the canonical start's ancestor chain.
- [x] #2 load_workspace_manifest uses the strict variant; an integration test plants a Cargo.toml at the symlink target of an ancestor and asserts the strict variant rejects it while the existing find_workspace_root keeps current behaviour.
<!-- AC:END -->
