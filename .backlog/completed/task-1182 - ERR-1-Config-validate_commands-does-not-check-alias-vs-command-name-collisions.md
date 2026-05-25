---
id: TASK-1182
title: >-
  ERR-1: Config::validate_commands does not check alias-vs-command-name
  collisions
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 08:10'
updated_date: '2026-05-10 06:29'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:119`

**What**: `validate_commands` walks composites for cycle/depth/unknown-ref but never verifies that an `aliases = ["build"]` entry does not collide with another command literally named `build`. The CLI's External dispatcher uses the literal name first, so the alias is silently dead — invisible to the user.

**Why it matters**: A configuration that "looks correct" never invokes the aliased command. Because validate_commands is the one place we promise to fail-loud on misconfiguration, the silent dead-alias is exactly the kind of trap the validator was introduced to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate_commands returns Err when any alias matches an existing command name (config-defined or in externals).
- [ ] #2 Regression test: a config with commands.build and commands.foo.aliases = [build] fails validate_commands, naming both keys.
<!-- AC:END -->
