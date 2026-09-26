---
id: TASK-2306
title: 'Config commands named like a builtin are silently shadowed: .ops.toml load never warns'
status: Triage
assignee: []
created_date: '2026-09-26 20:22'
labels:
  - code-review-rust
  - API
dependencies: []
modified_files:
  - crates/cli/src/main.rs
  - crates/cli/src/args.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs` (`builtin_subcommand_names`), config load in `crates/cli/src/main.rs:run`

**What**: `new-command` rejects names that collide with a builtin, and TASK-2297 pinned that no stack default collides — but a hand-written `[commands.init]` (or `tw`, `help`, `lock`, …) in `.ops.toml`, including one scaffolded by an older `ops init --commands` in a terraform workspace, loads without any diagnostic and is never reachable as `ops <name>`.

**Why it matters**: the command silently never runs; the user sees the builtin's behaviour instead. The shared `args::builtin_subcommand_names()` makes a load-time warning cheap.

**Origin**: discovered during TASK-2303 while fixing TASK-2297.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Loading a config whose command name collides with a builtin (name or alias) emits a warning naming the command and the builtin
<!-- AC:END -->
