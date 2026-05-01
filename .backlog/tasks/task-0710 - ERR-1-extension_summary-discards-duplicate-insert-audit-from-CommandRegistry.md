---
id: TASK-0710
title: 'ERR-1: extension_summary discards duplicate-insert audit from CommandRegistry'
status: Done
assignee:
  - TASK-0740
created_date: '2026-04-30 05:29'
updated_date: '2026-04-30 19:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:101-121`

**What**: `extension_summary` calls `ext.register_commands(&mut cmd_registry)` on a fresh `CommandRegistry` and reads `cmd_registry.keys()` to build the displayed command list. It never calls `cmd_registry.take_duplicate_inserts()`. Per TASK-0579 the registry tracks duplicate inserts so a self-shadow within one extension can be reported. The list/show paths in `extension_cmd.rs` therefore silently lose any duplicate command id from the rendered list (only the surviving entry appears) without emitting the warning that `register_extension_commands` (registry.rs:161-167) emits in the runner-wiring path.

**Why it matters**: `ops extension show <name>` and `ops extension list` are the operator-facing diagnostic surface. If an extension regresses to inserting the same id twice, the runner-wiring path warns but the `extension_summary` path stays silent — the operator looking at `ops extension show` sees a clean list and assumes the extension is healthy, defeating the purpose of the duplicate-tracking introduced by TASK-0579.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extension_summary drains take_duplicate_inserts() and emits a tracing::warn! per duplicate id naming the extension and the shadowed command
- [ ] #2 Test: an Extension that double-inserts the same command id must produce a captured WARN event when extension_summary is invoked, mirroring the assertion in register_extension_commands_warns_on_self_shadow
<!-- AC:END -->
