---
id: TASK-1137
title: >-
  CONC-3: register_commands does not detect alias collisions across config and
  stack stores
status: To Do
assignee:
  - TASK-1261
created_date: '2026-05-08 07:40'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:330-347`

**What**: `register_commands` warns on duplicate command-id registration but `merge_alias_for` (mod.rs:241-277) only inspects `non_config_alias_map` (stack + extension aliases). An extension whose alias collides with a config-defined alias from `config.resolve_alias` is silently shadowed at lookup time (config alias wins via `resolve_alias` ordering at resolve.rs:125), with no warn breadcrumb.

**Why it matters**: Operators reading `RUST_LOG=ops=debug` see id collisions but not alias collisions across stores. Audit-trail gap symmetric to the one TASK-0904 closed for command ids.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 merge_alias_for checks config.resolve_alias(alias) and emits tracing::warn! with both owners on collision
- [ ] #2 Unit test pins the warn line for a known cross-store alias collision
<!-- AC:END -->
