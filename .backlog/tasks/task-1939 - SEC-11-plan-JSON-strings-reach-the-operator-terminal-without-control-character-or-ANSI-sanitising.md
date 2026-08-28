---
id: TASK-1939
title: >-
  SEC-11: plan JSON strings reach the operator terminal without
  control-character or ANSI sanitising
status: To Do
assignee:
  - TASK-2002
created_date: '2026-08-27 15:47'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/render.rs
  - extensions-terraform/plan/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:139-148` and `:176-190`; values sourced in `extensions-terraform/plan/src/lib.rs:180-198` (`classify_plan`)

**What**: Every string taken out of the plan document is written straight into a table cell with no validation:

- `render_resource_table` at `:141-147` puts `c.resource_type`, `c.name` and `c.module` into `Cell::new(...)`
- `render_outputs_table` at `:189` puts the output map key (`name`) into `Cell::new(...)`
- `classify_plan` (`lib.rs:186-193`) clones `rc.r#type`, `rc.name`, `rc.module` and `rc.mode` verbatim out of the deserialized document

The document is untrusted in the sense that matters: `--json-file` accepts an arbitrary path (or `-` for stdin), and even on the default path the resource names come from third-party modules pulled from a registry. A `name` containing an ESC-bracket CSI sequence such as erase-line plus cursor-up (or a bare carriage return) can erase rows already printed, redraw a fake 'Plan: 0 to add, 0 to change, 0 to destroy.' line, or hide the 'WARNING: ... unrecognized' banner emitted at `:106-113` - on exactly the screen an operator reads before approving an apply. Embedded escapes also break comfy_table's width accounting, so column layout desynchronises.

Cross-crate note: the sink, `ops_core::table::OpsTable::cell` / `Cell::new` (`crates/core/src/table.rs`), does no sanitising either, so a shared helper there would be the better home for the fix - but the untrusted input enters the system in this crate, so the finding is filed here.

**Why it matters**: SEC-11 requires external input to be validated at the system boundary, including format and encoding. Terminal-escape injection into a change-review table is a spoofing primitive against a human approval step, not a cosmetic issue.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Resource type, name, module, mode and output names are stripped of ESC, CR and other C0/C1 control characters before rendering
- [ ] #2 The sanitising step is applied in one place, so a newly added column cannot forget it
- [ ] #3 A test renders a change whose name contains an escape sequence and a carriage return and asserts neither byte appears in the rendered output
- [ ] #4 A test renders an output whose key contains an escape sequence and asserts the same
<!-- AC:END -->
