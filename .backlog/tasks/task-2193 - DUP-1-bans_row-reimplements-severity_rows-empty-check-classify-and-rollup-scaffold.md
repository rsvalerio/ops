---
id: TASK-2193
title: 'DUP-1: bans_row reimplements severity_row''s empty-check, classify and rollup scaffold'
status: To Do
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-rust/deps/src/format.rs
priority: low
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:351`, `extensions-rust/deps/src/format.rs:297`

**What**: `bans_row` repeats `severity_row`'s opening verbatim in structure:

```rust
if bans.is_empty() { return ReportRow::new(ReportStatus::Ok, title, "None"); }
let classes: Vec<SeverityClass> = bans.iter().map(|b| SeverityClass::classify(&b.severity)).collect();
let (status, result) = rollup(&classes);
```

versus the same three steps at `severity_row:301-309`. The only genuine difference is the details body: bans render a single fixed `"(transitive, usually harmless)"` line instead of a per-entry table. `bans_row` also skips the unknown-severity warn that `severity_row` emits, so an unrecognised severity on a ban is classified as `Unknown` (and correctly rendered red / failed by the gate) with no log line — the drift breadcrumb exists for three of the four deny sections.

**Why it matters**: the empty-section representation, the classification call and the rollup are the parts most likely to change together across all four sections (they already changed together in TASK-0972 and TASK-1821). Having a fourth copy means the next change to the section header contract has to be made twice, and the missing warn shows the divergence has already happened once.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the empty-section return, the classify-and-rollup step and the unknown-severity warn are expressed once and shared by all four deny sections including bans
- [ ] #2 an unrecognised severity on a ban entry emits the same one-per-section drift warning the other sections emit
- [ ] #3 existing render tests for the Duplicate Crates row (status, result slot text, and the transitive detail line) still pass unchanged
<!-- AC:END -->
