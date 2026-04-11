---
id: TASK-023
title: inject_dynamic_commands exceeds 50-line threshold with repeated loop pattern
status: To Do
assignee: []
created_date: '2026-04-08 12:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-quality
  - CQ
  - FN-1
  - READ-6
  - low
  - effort-S
  - crate-cli
dependencies: []
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/main.rs:197-255`
**Anchor**: `fn inject_dynamic_commands`
**Impact**: At 58 lines, the function has two nearly identical loops (config commands at line 229, stack commands at line 241) that filter by builtins, dedup with `seen`, extract help text, and inject subcommands. The repeated pattern increases maintenance surface.

**Notes**:
Both loops share identical logic: `if builtins.contains(...) || !seen.insert(...) { continue; }` followed by help extraction and `cmd.subcommand(...)`. Extracting a helper like `inject_commands(cmd, commands_iter, builtins, seen)` would eliminate the duplication and bring the function under the 50-line threshold.

The function also uses `Box::leak` for `&'static str` conversion, which is documented and justified (runs once at process exit for help display).
<!-- SECTION:DESCRIPTION:END -->
