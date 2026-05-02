---
id: TASK-0842
title: >-
  ARCH-1: registry.rs is 832 lines mixing discovery, registration, audit, and
  helper APIs
status: Done
assignee: []
created_date: '2026-05-02 09:14'
updated_date: '2026-05-02 13:07'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:1-832`

**What**: One file owns: stack resolution (resolve_stack), compiled-extension collection, stack/config filtering, command-registry registration with collision audit, data-provider registration with collision audit, registry-builder convenience, and ref-conversion helpers. The audit-tracking logic for commands and data providers (~150 lines combined) repeats the same Owner<...> pattern.

**Why it matters**: ARCH-1 red flag is "module >500 lines with mixed responsibilities". Splitting registration.rs (the audit machinery) from discovery.rs (compiled-extension collection + filtering) would make the asymmetry between command/data paths visible by file structure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Split registry.rs into at least two submodules: one for compiled-extension discovery / filtering, one for the registration audit pipeline
- [x] #2 Extract the shared Owner machinery once
- [x] #3 cli crate public surface is unchanged (the registry::* re-exports keep callers unaffected)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Split crates/cli/src/registry.rs (815 lines) into a directory module: registry/mod.rs (re-exports), registry/discovery.rs (resolve_stack, collect_compiled_extensions, builtin_extensions, as_ext_refs, collect_extension_info), registry/registration.rs (Owner enum + seed_owners shared machinery, register_extension_commands, register_extension_data_providers, build_data_registry), and registry/tests.rs (existing test suite, unchanged). All 17 registry::* tests pass; ops verify clean. Public surface (crate::registry::*) unchanged.
<!-- SECTION:NOTES:END -->
