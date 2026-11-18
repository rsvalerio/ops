---
id: TASK-2745
title: 'TEST-11: provider_resolve_root_auto_discover_fails_without_cargo_toml asserts only is_err()'
status: To Do
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - testing
dependencies: []
parent_task_id: 'TASK-2850'
modified_files:
  - extensions-rust/cargo-toml/src/tests/provider.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions-rust/cargo-toml/src/tests/provider.rs:provider_resolve_root_auto_discover_fails_without_cargo_toml'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/provider.rs:220` (`provider_resolve_root_auto_discover_fails_without_cargo_toml`)

**What**: The test ends with a bare `assert!(result.is_err())`. Any error passes it (a context/registry problem, a read failure, a parse failure), not specifically "no workspace root found". The same file already documents (`provider.rs:63`) a case where a bare `is_err()` assertion passed or failed for reasons unrelated to the crate's logic.

**Why it matters**: The test cannot tell the intended failure (`FindWorkspaceRootError::NotFound` wrapped in the "resolving cargo_toml workspace root" context) from an unrelated regression, so a refactor that breaks discovery differently still passes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test asserts the error is the workspace-root-not-found case (e.g. downcasts to FindWorkspaceRootError and checks is_not_found(), or matches the context message)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.12 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
