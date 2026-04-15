---
id: TASK-0050
title: 'CD-8: Three-tier command lookup repeated in 4 CommandRunner methods'
status: Done
assignee: []
created_date: '2026-04-14 20:31'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-2
  - DUP-5
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/command/mod.rs:167-227`
**Anchor**: `fn resolve`, `fn canonical_id`, `fn resolve_alias`, `fn list_command_ids`
**Impact**: All four methods independently iterate the same three command stores (`config.commands`, `stack_commands`, `extension_commands`) with slight variations. Adding a new command source (e.g., workspace-local commands) would require updating all four methods. A shared `all_command_stores()` iterator or `find_in_stores(predicate)` helper would centralize the three-tier lookup.

DUP-2: 4 functions with similar structure differing only in the operation performed on each store. DUP-5: extract a shared helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper iterates all command stores; resolve, canonical_id, resolve_alias, and list_command_ids delegate to it
<!-- AC:END -->
