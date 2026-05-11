---
id: TASK-1290
title: 'API-1: OPS_NONINTERACTIVE/CI use var_os().is_some(), so =0 still opts in'
status: To Do
assignee:
  - TASK-1306
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:110-111`

**What**: `prompt_hook_install` decides non-interactive mode with `std::env::var_os(\"OPS_NONINTERACTIVE\").is_some() || std::env::var_os(\"CI\").is_some()`. Empty values, `=0`, or `=false` all trigger non-interactive mode — the opposite of user intent for `0`/`false`.

**Why it matters**: Users who set `OPS_NONINTERACTIVE=0` to mean \"off\" silently get the opposite behaviour. `CI=false` is sometimes set explicitly in tooling; that path also disables the prompt. Aligns badly with the `=truthy` ecosystem convention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Treat empty, '0', and 'false' as off for both OPS_NONINTERACTIVE and CI
- [ ] #2 Docstring lists the accepted truthy values
- [ ] #3 Test covers OPS_NONINTERACTIVE=0 keeping the prompt interactive
<!-- AC:END -->
