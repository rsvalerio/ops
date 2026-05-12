---
id: TASK-1373
title: >-
  API-1: AboutAction::Crates and AboutAction::Modules dispatch to a single
  ops_about::run_about_units implementation
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 21:42'
updated_date: '2026-05-12 23:29'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:43`

**What**: `run_about` matches `Some(AboutAction::Crates | AboutAction::Modules) => ops_about::run_about_units(&registry)`. The CLI surface exposes two distinct subcommands (`ops about crates`, `ops about modules`) with separate doc comments in `args.rs` ("Display crate cards." / "Display workspace modules (go.work / go.mod).") but both route to the same impl with no per-variant differentiation. From the user's perspective the variants must therefore either produce identical output (in which case one is dead) or `run_about_units` silently switches on something other than the chosen variant (in which case the dispatch is misleading).

**Why it matters**: Pattern-mediated CLI surface drift: the help string promises a different view per command, the dispatch promises a single behaviour, and the implementation crate (`ops_about::run_about_units`) has no way to know which user-facing variant was invoked. Either the variants need to thread their identity into the call, or one of them should be removed / aliased. As written, `ops about crates` and `ops about modules` are indistinguishable through this code path — and any future per-variant divergence would silently land in the wrong direction.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 either  accepts a variant discriminator and dispatches per-Crates / per-Modules view, or one of the two variants is removed / merged with the other in args.rs
- [ ] #2 the doc comments on  /  accurately describe what  produces for each variant
<!-- AC:END -->
