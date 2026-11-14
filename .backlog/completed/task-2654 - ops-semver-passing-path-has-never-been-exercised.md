---
id: TASK-2654
title: 'ops semver: passing path has never been exercised'
status: Done
assignee: []
created_date: '2026-11-01 00:45'
updated_date: '2026-11-01 18:28'
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
- [x] #1 ops semver has run against a real cargo-semver-checks install, on both a clean and a breaking change

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified 2026-11-01 against cargo-semver-checks 0.51.0 (real install). The ops repo itself cannot host the exercise: it publishes no crate (the crates.io "ops" is a foreign 2021 crate) and crates/cli is bin-only, which the tool skips. Instead ops semver ran in a clone of cfg-if at its published v1.0.5: clean checkout exits 0 (229 checks pass, baseline pulled from crates.io), and after removing #[macro_export] in a commit it exits 1 naming macro_no_longer_exported and "semver requires new major version". Probe, argument construction and exit-code mapping are correct; no code change needed. Also noted: ops semver works with no .ops.toml present.
<!-- SECTION:NOTES:END -->
