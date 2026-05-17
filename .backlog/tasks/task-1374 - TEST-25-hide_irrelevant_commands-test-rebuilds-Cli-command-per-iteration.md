---
id: TASK-1374
title: 'TEST-25: hide_irrelevant_commands test rebuilds Cli::command() per iteration'
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-12 21:46'
updated_date: '2026-05-17 09:25'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:368-398` (`hide_irrelevant_commands_preserves_non_stack_commands`)

**What**: The test loops over `result.get_subcommands()` and inside that loop calls `let original = Cli::command();` (line 385) per iteration to look up `was_hidden`. clap rebuilds the entire derived command tree (every subcommand, every arg, every help string) on every `Cli::command()` call — once per visible subcommand instead of once for the whole test. This is the same per-call expense flagged for production code in TASK-1318 / TASK-1368 (`PERF-1: validate_command_name rebuilds clap command tree per keystroke` and `Cli command invoked up to 3 times per CLI invocation`).

**Why it matters**: Tests inherit the same cost the production guards were filed to eliminate, so the regression that PERF-1 / PERF-3 work guards against is silently re-introduced in the test suite. The lookup `original.get_subcommands().find(...)` is the same data the test could compute once before the loop (or by inspecting `result` directly, since `hide_irrelevant_commands` only flips `hide`, it does not add or remove subcommands), and pre-building a `HashSet<&str>` of originally-hidden names is O(N) up-front vs O(N²) work inside the loop.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cli::command() is invoked at most once in hide_irrelevant_commands_preserves_non_stack_commands
- [x] #2 The was_hidden lookup uses a precomputed set keyed by name rather than a per-iteration linear scan over a rebuilt command tree
- [x] #3 Test assertion shape is unchanged (still asserts non-stack-specific previously-visible subcommands stay visible)
<!-- AC:END -->
