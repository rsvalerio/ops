---
id: TASK-2032
title: >-
  SEC-11: OpsTable does no control-character sanitising, so every table but the
  terraform plan one is unprotected
status: Done
assignee:
  - TASK-2044
created_date: '2026-08-28 21:30'
updated_date: '2026-08-29 12:33'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/table.rs
  - extensions-terraform/plan/src/render.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/table.rs`

**What**: `OpsTable::cell` and the re-exported `comfy_table::Cell::new` write
whatever bytes they are handed straight into a table cell. TASK-1939 fixed the
terraform plan tables by adding a `sanitize_terminal_text` / `text_cell` pair
inside `extensions-terraform/plan/src/render.rs`, because that is where the
untrusted input enters the system — but the sanitiser lives in that crate, so
every other `OpsTable` caller in the workspace still renders untrusted text raw.

**Why it matters**: an ESC-bracket CSI sequence in any value that reaches a table
can erase rows already printed and redraw fabricated ones, and it desynchronises
comfy-table's width accounting. `OpsTable` is the shared sink; a sanitising
constructor there (`OpsTable::text_cell`, say) would cover every table at once
and let `ops-tfplan` drop its local copy. The workspace also already carries
`ui::sanitise_line` for the non-table path, so a table-side equivalent should be
factored against it rather than added as a third implementation.

**Origin**: discovered during TASK-2002 while fixing TASK-1939, which noted the
cross-crate home explicitly but filed the finding against the ingress crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 OpsTable exposes a sanitising text-cell constructor that strips C0/C1 control characters, and it is the documented way to render untrusted text
- [x] #2 Callers rendering externally sourced strings into a table use that constructor
- [x] #3 extensions-terraform/plan/src/render.rs delegates to the shared helper instead of keeping its own sanitize_terminal_text
- [x] #4 A test asserts a control-sequence-bearing value rendered through OpsTable emits no ESC or CR byte
<!-- AC:END -->
