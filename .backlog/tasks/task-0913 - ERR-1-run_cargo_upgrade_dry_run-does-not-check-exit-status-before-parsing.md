---
id: TASK-0913
title: 'ERR-1: run_cargo_upgrade_dry_run does not check exit status before parsing'
status: Triage
assignee: []
created_date: '2026-05-02 10:11'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:22`

**What**: run_cargo_upgrade_dry_run captures stdout and parses it via parse_upgrade_table without inspecting output.status. A non-zero cargo exit (lockfile contention, network error, malformed Cargo.toml) leaves stdout empty/non-tabular, so parsing returns an empty Vec<UpgradeEntry> — i.e. no upgrades available for a failed invocation.

**Why it matters**: Mirrors the bug fixed for cargo-update in TASK-0502: the deps gate silently produces a clean dependency-health report when cargo upgrade actually crashed, hiding upstream problems from operators and CI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_cargo_upgrade_dry_run rejects non-zero exit status with an error containing the stderr tail
- [ ] #2 Unit test covers exit-code 1/101 cases and asserts an error is surfaced rather than an empty UpgradeResult
<!-- AC:END -->
