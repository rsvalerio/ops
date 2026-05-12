---
id: TASK-1375
title: >-
  PATTERN-1: prompt_hook_install returns ExitCode::from(130) directly, bypassing
  the ExitCodeOverride sentinel
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-12 21:46'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:142` (`prompt_hook_install`)

**What**: When the user presses Ctrl-C / Esc at the `Run \`ops <hook> install\` now?` confirm prompt, the handler returns `Ok(ExitCode::from(130))` with a magic-numbered SIGINT exit code inlined at the call site. The crate already defines `ExitCodeOverride(u8)` in `main.rs` (introduced by TASK-1293 specifically so fallible paths can bubble a specific exit code through `anyhow::Error`), with the inline doc on that type calling out SIGINT (130) and SIGPIPE (141) as the motivating cases. This site predates / duplicates that pattern: it returns the bare ExitCode rather than threading the override through anyhow, and it inlines the literal `130` rather than referencing a named SIGINT constant.

**Why it matters**: Two divergent conventions for "user cancelled, exit 130" now coexist — handlers that return `anyhow::Result<ExitCode>` versus handlers that return `anyhow::Result<()>` and instead attach `ExitCodeOverride(130)` to an error. A future refactor of `prompt_hook_install` to bail via `anyhow::bail!` (e.g. to surface a user-visible note) will silently drop the cancellation code unless the maintainer notices the magic literal. Same magic number ALSO appears in `classify_confirm_result` neighbours — centralising it in `main::ExitCodeOverride` or a `pub(crate) const SIGINT_EXIT: u8 = 130` keeps the SIGINT contract in one place. Related to but distinct from TASK-1325 (which addresses the misleading SUCCESS/FAILURE asymmetry between interactive-decline and non-interactive in the same function).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The SIGINT exit code (130) is referenced through a single named constant or through ExitCodeOverride, not inlined as a literal
- [ ] #2 prompt_hook_install's Ctrl-C / Esc path uses the same exit-code conduit as the rest of the CLI (either ExitCodeOverride attached to an anyhow error, or a shared SIGINT constant)
- [ ] #3 A grep for ExitCode::from(130) in crates/cli/src returns at most one occurrence (the constant definition itself), and the magic literal 130 is not repeated at SIGINT-meaning call sites
<!-- AC:END -->
