---
id: TASK-1361
title: >-
  ARCH: prompt_hook_install conflates non-interactive guard policy, UI emission,
  and hook dispatch in one helper
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:118`

**What**: `prompt_hook_install` bundles four concerns: (a) UI note emission, (b) non-interactive policy (`env_flag_enabled(OPS_NONINTERACTIVE)` + CI + TTY checks), (c) interactive Confirm prompt with cancellation classification, (d) hook dispatch. TASK-1322 (PATTERN-1) covers the stringly-typed dispatch — the orchestration tangle around it is the remainder of the smell.

**Why it matters**: The non-interactive guard policy is reusable for any future "ops install …" affordance, but it currently lives inline as 6 lines of `OPS_NONINTERACTIVE || CI || !is_tty` mixed with the inquire flow. Extracting `noninteractive_install_blocked(hook_name) -> Option<ExitCode>` (or similar) lets `prompt_hook_install` shrink to a Confirm + dispatch shell.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the non-interactive policy into a named helper (e.g. noninteractive_install_blocked) returning the early-exit ExitCode
- [ ] #2 Post-fix prompt_hook_install body is <20 lines and covers only prompt + dispatch; existing prompt tests still pass
<!-- AC:END -->
