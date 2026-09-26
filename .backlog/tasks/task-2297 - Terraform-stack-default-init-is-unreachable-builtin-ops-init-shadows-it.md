---
id: TASK-2297
title: 'Terraform stack default init is unreachable: builtin ops init shadows it'
status: Triage
assignee: []
created_date: '2026-09-26 19:06'
labels:
  - code-review-rust
  - API
dependencies: []
modified_files:
  - crates/core/src/.default.terraform.ops.toml
  - crates/cli/src/args.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/.default.terraform.ops.toml:1`

**What**: the terraform stack ships a `[commands.init]` default, but `ops init` is a clap builtin (`CoreSubcommand::Init`), and clap matches builtins before the `External` catch-all — so `ops init` in a terraform workspace creates `.ops.toml` instead of running `terraform init`. `validate_command_name` rejects user commands that collide with a builtin, but nothing checks stack defaults against the builtin set.

**Why it matters**: a documented stack default silently never runs; composites can still reference it by name, so the behaviour differs between `ops init` and a composite containing `init`.

**Origin**: discovered during TASK-2290 while fixing TASK-2280 (the same collision is why the plan inspector shipped as `ops explain` rather than `ops plan`, since terraform also ships `plan`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No stack default command name collides with a clap builtin subcommand (renamed, or the collision reported)
- [ ] #2 A test pins that every stack default name is reachable as ops <name>
<!-- AC:END -->
