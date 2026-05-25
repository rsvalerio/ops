---
id: TASK-1285
title: 'ARCH-3: build_runner accepts an unused _verbose: bool parameter'
status: Done
assignee:
  - TASK-1305
created_date: '2026-05-11 15:26'
updated_date: '2026-05-11 18:23'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:76-86`

**What**: `build_runner` declares `_verbose: bool` but never reads it; verbose is consumed later by `ProgressDisplay`, not the `CommandRunner`. Both call sites still thread the flag through to this function.

**Why it matters**: Dead bool parameters next to other bools recreate exactly the swap-bug footgun `RunOptions`/`PlanShape` were introduced to eliminate. A future refactor could plausibly wire the wrong bool into this slot without any compile-time signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove _verbose from build_runner's signature and both call sites, or wire it through to a real runner-side setting if it was intended to land
- [ ] #2 build_runner's signature reflects only knobs the runner itself consumes
<!-- AC:END -->
