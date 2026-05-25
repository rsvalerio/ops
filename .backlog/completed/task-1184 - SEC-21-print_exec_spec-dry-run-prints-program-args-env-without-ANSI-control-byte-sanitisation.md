---
id: TASK-1184
title: >-
  SEC-21: print_exec_spec dry-run prints program/args/env without
  ANSI/control-byte sanitisation
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 08:10'
updated_date: '2026-05-08 13:31'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/dry_run.rs:57`

**What**: `writeln!(w, "      program: {}", vars.try_expand(&e.program)?)?;` writes the (possibly env-expanded) program string verbatim to stdout. A .ops.toml with `program = "evil\u{1b}[2J\u{1b}[H"` (or expansion of an attacker-controlled \${VAR} containing ANSI) clears the operator's screen during ops --dry-run.

**Why it matters**: Dry-run is the audit channel — it exists so an operator can review what would run before running it. ANSI/control bytes in program, args, env values, or cwd round-trip to the terminal and can hide commands above the cleared region or forge fake "Done" lines, defeating the audit. ui.rs::sanitise_line already exists for this exact threat model on stderr.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All values printed by print_exec_spec (program, each arg, each env value, cwd) flow through a control-byte sanitiser before reaching the writer.
- [x] #2 Regression test: a spec with program = x\u{1b}[2Jy produces dry-run output containing escaped form and no raw ESC byte.
<!-- AC:END -->
