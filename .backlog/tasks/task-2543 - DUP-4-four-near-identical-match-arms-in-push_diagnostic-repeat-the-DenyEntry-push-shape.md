---
id: TASK-2543
title: 'DUP-4: four near-identical match arms in push_diagnostic repeat the DenyEntry push shape'
status: Done
assignee: []
created_date: '2026-10-10 15:37'
updated_date: '2026-10-10 21:46'
labels:
  - code-review
  - dup
dependencies: []
parent_task_id: 'TASK-2618'
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-4:extensions-rust/deps/src/parse/deny.rs:push_diagnostic'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:548`

**What**: Four match arms in `push_diagnostic` are near-identical 5-line blocks: `DiagClass::License`, `DiagClass::UnusedLicenseAllowance`, `DiagClass::Ban` and `DiagClass::Source` each push `Wrapper(DenyEntry { package, message: diag.message, severity: diag.severity })` into their section vec, differing only in the target field and the wrapper constructor. The `Advisory` arm is the only one with distinct shaping.

**Why it matters**: DUP-4 (near-identical match arms in one function). A fifth diagnostic class would copy the block a fifth time, and a field added to `DenyEntry` must be repeated in four places. Since `LicenseEntry`, `BanEntry` and `SourceEntry` all wrap `DenyEntry`, a shared helper (or `From<DenyEntry>` impls plus one generic push) collapses the four arms into data: a `(wrapper, target vec)` table or a small generic fn.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The License/UnusedLicenseAllowance/Ban/Source arms no longer each repeat the full DenyEntry construction; the shape is expressed once
- [x] #2 Existing deny parser tests pass unchanged

<!-- AC:END -->
