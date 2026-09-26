---
id: TASK-2301
title: 'Global --dry-run is still ignored by builtin subcommands that write or spawn (tw, eof, init, lock, import-makefile, new-command, theme select, about setup)'
status: To Do
assignee: []
created_date: '2026-09-26 19:13'
updated_date: '2026-09-26 19:55'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2303'
modified_files:
  - crates/cli/src/main.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/lock_cmd.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:252`

**What**: `dispatch` threads `cli.dry_run` only into `sec`, `clippy-findings`, the External run path, the hook runners and (since TASK-2278/TASK-2279) `backlog`. Every other builtin accepts the global `--dry-run` and ignores it: `ops --dry-run trailing-whitespace` / `end-of-file-fixer` rewrite files, `ops --dry-run lock <name> -- <cmd>` runs the command, and `init`, `new-command`, `import-makefile`, `theme select`, `about setup` write config.

**Why it matters**: `--dry-run` is documented as "Preview commands without executing"; the same class of bug as TASK-2278 (a preview that rewrites files). The flag must either preview or be refused per subcommand, never silently ignored.

**Origin**: discovered during TASK-2290 while fixing TASK-2278 / TASK-2279.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every builtin subcommand either honours the global --dry-run (no write, no spawn) or rejects it with an explicit error
- [ ] #2 A test enumerates the builtin subcommands so a new one must be classified
<!-- AC:END -->
