---
id: TASK-1510
title: >-
  ERR-4: CargoTomlProvider::resolve_root propagates find_workspace_root error
  without context
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 19:57'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:199-204`

**What**: `resolve_root` returns `Ok(find_workspace_root(working_dir)?)` and propagates the typed `FindWorkspaceRootError` directly through anyhow without `.with_context(...)` documenting the working directory that triggered the discovery. Callers (notably `provide_typed` -> `provide` -> `DataProvider::provide` -> `Context::get_or_provide`) ultimately see the bare `FindWorkspaceRootError::NotFound { start, depth }`, but lose the higher-level breadcrumb "while resolving the cargo_toml data provider's workspace root for <working_directory>". Inconsistent with the rest of `provide_typed`, which uses `.with_context(|| format!("reading {}", ...))` on every other `?`. ERR-4 mandates context on every `?` propagation in library code.

**Why it matters**: Provider failures bubble up through `DataProvider::provide` and surface to operators as terse "no Cargo.toml found in /tmp/foo or any parent directory" lines with no provider attribution. When multiple data providers cohabit a request, the operator cannot tell which provider is failing without grepping. The cost is one `.with_context(|| format!("resolving cargo_toml workspace root from {}", working_dir.display()))`.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_root attaches an anyhow context describing the provider and the working directory it walked from
- [ ] #2 context survives through provide / provide_typed without being shadowed by the inner .with_context calls
- [ ] #3 existing tests in src/tests/find_root.rs and src/tests/provider.rs continue to pass
<!-- AC:END -->
