---
id: TASK-2088
title: >-
  READ-12: three runner log sites interpolate preformatted or duplicate values
  into the message
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/finalize.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:333-338`; `crates/runner/src/display.rs:291-298`; `crates/runner/src/display/finalize.rs:68-71`

**What**: Three logging sites in this crate still use the positional/preformatted dialect the rest of the workspace has moved off (cf. TASK-2070 for the same drift in extensions/about):
- exec.rs:333 — the drain-deadline warn already carries a named `grace_secs` field but ALSO interpolates `{}s` positionally in the message with the same value, so the rendered text duplicates the field.
- display.rs:292-297 — StepOutputDropped handler builds `line` via `format!` before checking `tracing::enabled!`, so the allocation happens even when debug is off and nothing is emitted (PERF-19); the `format!` result is then logged positionally via `"{line}"` and only the enabled-check reuse needs the String.
- finalize.rs:69 — `report_tap_truncation` logs the preformatted `line` positionally (`"{}", line`) instead of named fields (step_id, kind are already in scope as separate values).

**Why it matters**: READ-12 — a value baked into the message text cannot be filtered or aggregated on by a subscriber; the exec.rs site additionally says the same thing twice in one record. The display.rs site also pays an unconditional per-drop `format!` on the event-pump path, which PERF-19 flags as exactly the cost structured emission avoids.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 exec.rs drain warn keeps the named grace_secs / env fields and drops the positional duplicate from the message template
- [ ] #2 display.rs builds the drop-count line only inside the enabled check (or emits structured fields and formats just for the stderr/tap mirror)
- [ ] #3 finalize.rs warn carries step_id and kind as named fields with a stable message template
<!-- AC:END -->
