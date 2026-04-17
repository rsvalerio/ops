---
id: TASK-0074
title: 'DUP-1: dry-run writeln! block duplicates ExecCommandSpec::display_cmd'
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 15:12'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:223`

**What**: print_exec_spec manually re-expands vars.expand(&e.program), .args, .cwd and prints a formatted block; the same ExecCommandSpec::display_cmd logic is used elsewhere.

**Why it matters**: Two representations of the same what-would-this-command-run-as output drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reuse ExecCommandSpec::display_cmd() (or extract a shared formatter) for program/args part
- [ ] #2 Keep only env/cwd/timeout lines unique to dry-run
<!-- AC:END -->
