---
id: TASK-0576
title: >-
  ERR-7: Variables::expand silently swallows expansion errors in dry-run/display
  paths
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:16'
updated_date: '2026-04-29 10:54'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:76`

**What**: `Variables::expand` (the lossy variant) catches every `ExpandError`, logs at `tracing::warn!`, and returns `Cow::Borrowed(input)` so the literal `${VAR}` flows to display. Callers in `cli/src/run_cmd/dry_run.rs:54,81,95` and `core/src/config/commands.rs:152` use this for dry-run output and `expanded_args_display`, where the user is asking "what command will run".

**Why it matters**: ERR-7/ERR-1. The strict try_expand exists for spawn (TASK-0450) but dry-run callers still use the silent variant, so a user inspecting their config never sees the same diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 dry_run::print_exec_spec and expanded_args_display switch to try_expand and propagate the error (preferred), or lossy variant returns visible sentinel
- [x] #2 Dry-run with non-UTF-8 env var produces user-visible diagnostic (not just tracing event)
<!-- AC:END -->
