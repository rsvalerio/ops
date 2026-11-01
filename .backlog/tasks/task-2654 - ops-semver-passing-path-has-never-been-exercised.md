---
id: TASK-2654
title: 'ops semver: passing path has never been exercised'
status: Triage
assignee: []
created_date: '2026-11-01 00:45'
labels:
  - code-review-rust
  - test
dependencies: []
modified_files:
  - crates/cli/src/cargo_check_cmd.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/cargo_check_cmd.rs`

**What**: `ops semver` was verified only down its missing-tool path, because cargo-semver-checks is not installed on the machine that built it. A real run of cargo-semver-checks through the command has not happened.

**Why it matters**: the feature's core behaviour shipped untested; a wrong argument or exit-code mapping would surface only for the first user with the tool installed.

**Origin**: discovered during TASK-2645 while fixing TASK-2639; surfaced in the run-level report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops semver has run against a real cargo-semver-checks install, on both a clean and a breaking change
<!-- AC:END -->
