---
id: TASK-2649
title: 'Commands: a spelling for re-invoking the running ops from .ops.toml (wrap around a group)'
status: Triage
assignee: []
created_date: '2026-11-01 00:30'
labels:
  - code-review-rust
  - config
dependencies: []
modified_files:
  - crates/core/src/config/commands.rs
  - docs/configuration.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/configuration.md` (Running a command inside a wrapper), `crates/core/src/config/commands.rs:106`

**What**: The documented way to keep one service lifetime across several steps is `program = "ops"`, `args = ["<group>"]` with `wrap`. A bare `ops` in config resolves through `PATH`, while internal registrations spawn `current_ops_program()` for exactly that reason. The wrapper can also change `PATH` before the re-invocation.

**Why it matters**: The re-invoked binary can differ from the running one (a stale `~/.cargo/bin` copy, a direnv shim), so the steps run under another ops version. `.ops.toml` already runs under the make trust model, so this is a correctness gap, not a sandbox escape.

**Origin**: discovered during TASK-2644 while fixing TASK-2641.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A config can name the running ops binary without resolving it through PATH, or the docs state why it cannot
<!-- AC:END -->
