---
id: TASK-1026
title: >-
  ERR-1: parse_upgrade_table relies on case-sensitive header substring
  'name'+'old req' that breaks on cargo-edit format drift
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-08 06:36'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:96-99`

**What**: `parse_upgrade_table` decides a row is the cargo-upgrade header via `line.trim_start().starts_with("name") && line.contains("old req")`. The separator-row dispatch (line 102) requires `starts_with("====")`. Both checks are exact, case-sensitive substring matches against cargo-edit's current rendering. If a future cargo-edit changes the header capitalisation (e.g. `Name`, `Old Req`), localizes it, or renames the column (the table format is not a stable API of cargo-edit), the header-detection silently fails — `columns` stays `None`, `parse_upgrade_row` is never reached, and `parse_upgrade_table` returns an empty Vec.

The previous TASK-0913 fix added an exit-code guard that catches non-zero exits, but cargo-upgrade emits exit 0 with the new (unrecognised) header format, so the empty-Vec result rolls forward as 'no upgrades available' with zero log signal.

**Why it matters**: `ops deps` is the supply-chain visibility surface — silently reporting 'no upgrades' is exactly the failure mode the deps gate exists to prevent. This is the same fail-open class as TASK-0958 (cargo-deny zero diagnostics on exit 1), but for cargo-upgrade.

**Suggested fix**: when stdout has non-empty body lines but no separator row was detected, emit a `tracing::warn!` flagging suspected header format drift, or fail the parse closed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_upgrade_table emits a tracing::warn when stdout contains non-empty lines but no separator row was detected, OR returns Err so the gate fails closed
- [x] #2 Unit test feeds a header-renamed fixture and asserts the warn/Err path fires
<!-- AC:END -->
