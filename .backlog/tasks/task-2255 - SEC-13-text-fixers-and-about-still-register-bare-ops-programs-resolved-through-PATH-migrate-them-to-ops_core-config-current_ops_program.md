---
id: TASK-2255
title: >-
  SEC-13: text-fixers and about still register bare 'ops' programs resolved
  through PATH; migrate them to ops_core::config::current_ops_program
status: Triage
assignee: []
created_date: '2026-09-08 17:49'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
  - extensions/about/src/lib.rs
priority: medium
ordinal: 161000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/lib.rs:95`, `extensions/text-fixers/src/lib.rs:101`, `extensions/about/src/lib.rs:69`

**What**: TASK-2122 (code-review wave0) fixed config-checkers' `check-json`/`check-yaml` registration to spawn the absolute `current_exe()`-derived path via the new shared helper `ops_core::config::current_ops_program()` (with `display_program = Some("ops")` so the rendered step line is unchanged). The two sibling extensions named in that task still register `ExecCommandSpec::new("ops", [...])` — a bare program name that `std::process::Command` resolves through the invoking environment's `PATH`, so an `ops` shim earlier on `PATH` silently becomes the binary that runs, and a freshly-built `./target/debug/ops` invokes the *installed* `ops` for these steps (version skew).

**Why it matters**: the integrity and version-skew arguments from TASK-2122 apply verbatim; only the crate under review scoped that fix to config-checkers. The shared helper now exists, so the migration is mechanical: resolve `current_ops_program()`, set `display_program = Some("ops")`, and pin with a registration test asserting the program is absolute and the display line still reads `ops <subcommand>` (mirroring `registered_checkers_spawn_absolute_ops_and_display_as_ops` in `extensions/config-checkers/src/tests.rs`).

**Origin**: discovered during TASK-2234 (wave0) while fixing TASK-2122.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 text-fixers and about register every ops re-invocation with an absolute current_exe()-derived program and display_program = "ops"
- [ ] #2 a registration test per crate asserts the program is absolute when current_exe() succeeds and the rendered step line still reads ops <subcommand>
<!-- AC:END -->
