---
id: TASK-1202
title: >-
  ERR-1: parse_upgrade_row downgrades column-shape mismatches to tracing::debug,
  hiding cargo-edit drift
status: Done
assignee:
  - TASK-1267
created_date: '2026-05-08 08:16'
updated_date: '2026-05-09 14:42'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:198-238`

**What**: When the separator row indicates fewer than 5 columns, or when the row body fails to fill the 5 fixed columns, parse_upgrade_row returns None after a tracing::debug! breadcrumb. Sister format-drift signals in the same module (TASK-1026, TASK-1074) fire at tracing::warn. Row-level drift never reaches interpret_upgrade_output's diagnostics — it produces a short Vec<UpgradeEntry> with the malformed rows silently missing.

**Why it matters**: The whole point of the surrounding TASK-0913 / TASK-1026 / TASK-1074 work is that the cargo-edit table format is unstable. Row-shape mismatch is exactly the same drift class as the header-token check, and operators running default RUST_LOG see neither the missing rows nor the debug breadcrumb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A row whose separator advertises >=5 columns but where any of the first 5 columns trims to empty produces a tracing::warn (or contributes to a counter that interpret_upgrade_output checks).
- [x] #2 The TASK-1074 / TASK-1026 fail-closed path covers 'saw_recognised_header && saw_separator && body_lines > 0 && entries == 0' so a wholesale row-shape drift surfaces as an error rather than an empty Vec.
<!-- AC:END -->
