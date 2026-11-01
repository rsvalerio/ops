---
id: TASK-2651
title: 'ops config check reads only mise.toml''s bare ops key'
status: Triage
assignee: []
created_date: '2026-11-01 00:41'
labels:
  - code-review-rust
  - config
  - ci
dependencies: []
modified_files:
  - crates/core/src/config/history.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/history.rs:1` (`mise_ops_pin`)

**What**: The pin is read from `tools.ops` in `mise.toml` only. mise also reads `.mise.toml` and `.config/mise.toml`, and a project can pin the tool under its backend key `"github:rsvalerio/ops"` with no `[tool_alias]`.

**Why it matters**: In those layouts `ops config check` finds no pin, prints only the minimum and exits 0, so a real pin mismatch passes. `ops init --rust --check` shares the lookup.

**Origin**: discovered during TASK-2646 while fixing TASK-2638.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every pin location mise honours for ops is read, or the supported set is documented
<!-- AC:END -->
