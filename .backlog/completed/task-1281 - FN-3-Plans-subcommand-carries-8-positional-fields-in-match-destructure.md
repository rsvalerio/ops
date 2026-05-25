---
id: TASK-1281
title: 'FN-3: Plans subcommand carries 8 positional fields in match destructure'
status: Done
assignee:
  - TASK-1305
created_date: '2026-05-11 15:26'
updated_date: '2026-05-11 18:20'
labels:
  - code-review-rust
  - function
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:131-156` and `crates/cli/src/main.rs:215-235`

**What**: `CoreSubcommand::Plans { json_file, out, json_out, keep_plan, no_color, detailed_exitcode, show_outputs, passthrough }` has 8 fields, and dispatch then unpacks all 8 just to repack them into `ops_tfplan::PlanOptions { ... }` with identical names.

**Why it matters**: Both the variant struct definition and the destructure/repack mirror the PlanOptions shape, so every new plan flag costs three edits across two files. Either parse directly into a `clap_derive`-flattened PlanOptions (`#[command(flatten)]`-style or a `#[derive(Args)]` struct), or have the variant hold a `PlanOptions` directly so dispatch is `run_plan_pipeline(opts)`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CoreSubcommand::Plans either embeds a single struct argument (e.g. Plans(PlanArgs)) or PlanOptions is constructed via From<PlansArgs>/#[command(flatten)]
- [ ] #2 dispatch arm reduces to one line forwarding the struct
- [ ] #3 Adding a new plan flag requires touching exactly one definition site
- [ ] #4 Existing parse tests still pass without renaming fields
<!-- AC:END -->
