---
id: TASK-1331
title: 'FN-3: run_hook_action takes adjacent (is_install, changed_only) bool params'
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-12 16:26'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:201-212`

**What**: `run_hook_action(config, hook, is_install, changed_only)` keeps two adjacent boolean parameters of unrelated meaning. The codebase already replaced similar patterns with named structs/enums elsewhere (see `PlanShape`/`RunOptions`); the hook dispatch path regressed back to bare bools.

**Why it matters**: Adjacent same-typed bool args are a swap footgun — the compiler cannot catch a caller that transposes them. An enum like `HookAction::{Install, Run { changed_only }}` would also remove the dead `changed_only` value carried through Install dispatch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Booleans replaced with a named struct/enum so transposed callers fail to compile.
- [ ] #2 Hook install vs run paths share no irrelevant fields.
<!-- AC:END -->
