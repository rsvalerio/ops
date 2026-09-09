---
id: TASK-2179
title: 'ERR-1: cargo-upgrade row-shape drift is only caught when every row fails, so partial row loss silently shrinks the upgrade list'
status: Done
assignee: []
created_date: '2026-09-08 07:12'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions-rust/deps/src/parse/upgrade.rs
priority: medium
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:122`

**What**: `check_row_shape_drift` fires only on total loss:

```rust
if diag.saw_recognised_header && diag.saw_separator && diag.body_lines > 0 && diag.entries_emitted == 0
```

`parse_upgrade_row` drops a row (returning `None`, logged at `debug`) whenever `slice_fixed_columns` cannot fill all five columns — e.g. a row whose values shifted past a column edge, or a row where one of the five slots came back empty after trimming. If 9 of 10 rows drop, `entries_emitted == 1`, the guard stays silent, and `ops deps` reports one available upgrade where there are ten. Nothing above logs at warn level.

The sibling parser already solved exactly this: `parse/deny.rs` counts `candidate_diagnostics` against `entries_emitted` and bails via `check_partial_decode_loss` once the dropped share exceeds `MAX_DROPPED_SHARE_NUM / MAX_DROPPED_SHARE_DEN` (25%), on the reasoning that partial class loss is the *likely* shape of upstream drift. The upgrade parser has the same two counters available (`body_lines`, `entries_emitted`) and does not use them for anything but the all-or-nothing case.

**Why it matters**: a partially-dropped table understates available upgrades, including security-relevant ones, with a green report and no warn-level breadcrumb — the same fail-open posture the crate spent TASK-1074 / TASK-1202 / TASK-1817 / TASK-1840 eliminating everywhere else.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check_row_shape_drift (or a sibling guard) fails closed when the share of body_lines that produced no UpgradeEntry exceeds a named threshold constant, not only when entries_emitted is zero
- [ ] #2 the threshold is a documented const with a rationale, mirroring MAX_DROPPED_SHARE_NUM / MAX_DROPPED_SHARE_DEN in parse/deny.rs
- [ ] #3 a test feeds a recognised header, a separator, and a table where most rows fail the 5-column shape while one parses, and asserts interpret_upgrade_output errs rather than returning the survivor
- [ ] #4 existing tests covering a legitimately small table and a single skipped malformed row still pass
<!-- AC:END -->
