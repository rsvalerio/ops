---
id: TASK-0020
title: CommandRunner uses inconsistent map types for command sources
status: Done
assignee: []
created_date: '2026-04-10 23:30:00'
updated_date: '2026-04-11 09:57'
labels:
  - rust-code-quality
  - CQ
  - READ-6
  - low
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/command/mod.rs:76-84`
**Anchor**: `struct CommandRunner`
**Impact**: `CommandRunner` stores three command sources using two different map types: `config: Arc<Config>` (whose `commands` field is `IndexMap`), `stack_commands: IndexMap<CommandId, CommandSpec>`, and `extension_commands: std::collections::HashMap<CommandId, CommandSpec>`. The first two preserve insertion order; the third does not. All three serve the same logical purpose (command ID → spec lookup) and are iterated in `list_command_ids` (line 217-224) and searched in `resolve` (line 164-171), `canonical_id` (line 176-197), and `resolve_alias` (line 200-214).

**Notes**:
READ-6: "Consistent patterns for similar problems."

The inconsistency has no correctness impact — `list_command_ids` sorts the merged results anyway (line 221), and `resolve` checks sources in priority order regardless of map type. But it adds cognitive friction: a reader must reason about whether insertion order matters for extension commands and why it differs from the other two sources.

Fix: Change `extension_commands` to `IndexMap<CommandId, CommandSpec>` for consistency. The `indexmap` crate is already a dependency. This also aligns with `CommandRegistry` (defined as `IndexMap<CommandId, CommandSpec>` in `extension/src/lib.rs:91`), which is what extensions populate before handing to `register_commands`.
<!-- SECTION:DESCRIPTION:END -->
