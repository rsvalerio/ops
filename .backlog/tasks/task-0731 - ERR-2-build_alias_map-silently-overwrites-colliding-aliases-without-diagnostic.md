---
id: TASK-0731
title: >-
  ERR-2: build_alias_map silently overwrites colliding aliases without
  diagnostic
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:49'
updated_date: '2026-04-30 18:08'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:187-200` (`build_alias_map`)

**What**: `build_alias_map` flattens stack + extension command stores into a single `HashMap<alias, canonical>`. Two stores can register the same alias for different canonical commands (e.g. stack defaults define `clippy` aliasing `lint`, an extension also aliases `lint`). The function uses `map.insert(alias.clone(), name.to_string())` and discards the previous entry — the late writer wins silently.

This diverges from the duplicate-detection policy applied to:
  - `CommandRegistry::insert` (records duplicate inserts; warned by `cli/src/registry.rs:163`)
  - `register_extension_commands` (warns cross-extension shadows, `cli/src/registry.rs:174`)
  - `CommandRunner::register_commands` (warns same-id reinsert, `runner/src/command/mod.rs:208`)

A user adding an alias collision via an extension would observe a different canonical command being run from what their stack defines, with zero log output explaining why.

**Why it matters**: Aliases are the user-facing surface for command resolution; silent shadowing is exactly the symptom CL-5 calls out. The existing infrastructure (TASK-0402 / TASK-0579) already established that registry collisions must surface a `tracing::warn!`; aliases are the matching gap.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 build_alias_map detects alias collisions across stores and emits a tracing::warn! with the offending alias and both canonical owners
- [ ] #2 Decide and document the resolution policy (last-write-wins vs first-write-wins) so it matches CommandRegistry / DataRegistry consistently — see CL-5 / TASK-0661
<!-- AC:END -->
