---
id: TASK-2335
title: 'Report tool versions (installed or minimum) alongside ops explain --json tools'
status: Triage
assignee: []
created_date: '2026-09-28 15:19'
labels:
  - code-review-rust
  - ci
  - ops-alignment
dependencies: []
modified_files:
  - crates/cli/src/run_cmd/tools.rs
  - extensions-rust/deps/src/lib.rs
  - docs/commands.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tools.rs:1`

**What**: TASK-2326 asked for the external tools each command needs "with versions". `ops explain --json` now lists the tools (`tools[]`: name, optional, install, requiredBy) but no version: explain is spawn-free by contract, and ops pins no minimum versions for cargo-nextest, cargo-deny, trivy, etc.

**Why it matters**: forge CI installers (forge TASK-0026/TASK-0030) may need a pinned or minimum version to install reproducibly; without it they still hand-maintain versions.

**Origin**: discovered during TASK-2333 while fixing TASK-2326.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each reported tool carries a minimum/pinned version when ops declares one, or an opt-in probe reports the installed version without making plain ops explain spawn
<!-- AC:END -->
