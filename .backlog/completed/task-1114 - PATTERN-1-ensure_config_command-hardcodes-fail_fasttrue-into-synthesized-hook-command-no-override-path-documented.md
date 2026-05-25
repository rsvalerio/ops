---
id: TASK-1114
title: >-
  PATTERN-1: ensure_config_command hardcodes fail_fast=true into synthesized
  hook command, no override path documented
status: Done
assignee: []
created_date: '2026-05-07 21:51'
updated_date: '2026-05-08 06:17'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/config.rs:55`

**What**: `ensure_config_command` writes `cmd.insert("fail_fast", toml_edit::value(true));` unconditionally when synthesizing the `[commands.<hook>]` entry in `.ops.toml`. The choice of `true` is undocumented at the call site and there is no `HookConfig` field, env var, or CLI flag that would let an operator install a hook whose composite command is allowed to keep going past a failing sub-command.

**Why it matters**: `fail_fast` materially changes the semantics of the hook: with `true`, the first failing sub-command short-circuits and the rest never run; with `false`, every sub-command runs and the hook reports the union of failures. Both are reasonable defaults, but baking one in without a knob means the operator-visible workaround is "delete the synthesized entry, rerun install, hand-edit .ops.toml" — and the early-exit guard at the top of `ensure_config_command` makes the latter sticky once written. At minimum the choice belongs in `HookConfig` (per-extension policy) and the user-facing install flow should mention how to flip it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 fail_fast is configurable through HookConfig (or equivalent) rather than a hardcoded literal, OR the install path documents the policy and how to override it
<!-- AC:END -->
