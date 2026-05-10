---
id: TASK-0698
title: >-
  ERR-2: Stack::default_commands panics on malformed embedded TOML at production
  call sites
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:26'
updated_date: '2026-04-30 18:04'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:165-176`

**What**: `default_commands()` calls `toml::from_str(toml).expect("stack default commands TOML must be valid")` on the embedded `.default.<stack>.ops.toml`. Production callers reach this via `init_template -> stack.default_commands()` (config/mod.rs:343-347) and `extensions::resolve_stack -> default_commands()`. The accompanying test `all_embedded_default_tomls_parse` is the only safety net; if a future PR ships a malformed default TOML and bypasses CI (broken job, force-merge), the panic surfaces during user `ops init` as an opaque process abort.

**Why it matters**: A binary that panics during init is significantly worse than one that returns a typed error and falls back to an empty default-command map. The existing `Result`-based `validate()` infrastructure in `Config` already shows the right shape for this kind of fail-loud-but-recoverable diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Convert default_commands to Result<IndexMap<...>, anyhow::Error> or log+return empty
- [ ] #2 Update call sites to surface the error via the existing anyhow::Result chain
- [ ] #3 Keep all_embedded_default_tomls_parse test as compile-time/unit-time guard
<!-- AC:END -->
