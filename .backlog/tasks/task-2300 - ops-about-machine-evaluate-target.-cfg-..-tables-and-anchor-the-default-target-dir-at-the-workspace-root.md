---
id: TASK-2300
title: 'ops about machine: evaluate target.''cfg(..)'' tables and anchor the default target dir at the workspace root'
status: To Do
assignee: []
created_date: '2026-09-26 19:08'
updated_date: '2026-09-26 19:55'
labels:
  - code-review-rust
  - feature
dependencies: []
parent_task_id: 'TASK-2304'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs`

**What**: `resolve_cargo_settings` reads `target.<host-triple>.linker/rustflags` but not `target.'cfg(...)'` tables, so a linker/rustflags set via a cfg table is reported as unset. The default target dir is `<cwd>/target`, which is wrong when `ops about machine` runs from a member subdirectory (cargo uses the workspace root).

**Why it matters**: `rust-make-build-fast` records these next to timings; a missed cfg-table linker or a wrong target-dir filesystem makes two timings look comparable when they are not.

**Origin**: discovered during TASK-2292 while fixing TASK-2287.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cfg(...) target tables matching the host are evaluated for linker and rustflags, with their source
- [ ] #2 The default target dir resolves against the cargo workspace root, not the cwd
<!-- AC:END -->
