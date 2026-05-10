---
id: TASK-0699
title: >-
  ERR-2: merge_indexmap silently overwrites existing keys without diagnostic for
  command/theme/tool collisions
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:27'
updated_date: '2026-04-30 18:04'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:13-20`

**What**: `merge_indexmap` calls `base.extend(items)`, which for `IndexMap` overwrites existing entries with the same key without surfacing the collision. This is invoked during `merge_config` for `commands`, `themes`, and `tools` (merge.rs:63, 65, 74). A user whose `~/.config/ops/config.toml` defines `[commands.test]`, then a project `.ops.toml` also defining `[commands.test]`, gets the project version with no log signal. Sibling registries (CommandRegistry post-TASK-0579, DataRegistry post-TASK-0350) explicitly warn on duplicate inserts; the configuration merge layer inverts that policy.

**Why it matters**: Layered config is exactly where shadowing is most surprising — users who set a command at one level and "lose" it at another have no signal. The merge layer should at minimum emit a `tracing::debug` event listing replaced keys per source so `OPS_LOG_LEVEL=debug` reveals the resolution.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit a tracing::debug event with replaced keys when an overlay shadows base entries
- [ ] #2 Document the merge=overwrite semantics on merge_config and the per-section docs
- [ ] #3 Add a test that asserts the debug event fires for a colliding command/theme/tool key
<!-- AC:END -->
