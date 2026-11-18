---
id: TASK-2794
title: 'API-14: public about subpage entry points and document fields lack summary docs'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/units.rs
  - extensions/about/src/coverage.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/code.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/about/src/units.rs:run_about_units'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:22-26` (`run_about_units`), `:35-50` (`run_about_units_with`); `extensions/about/src/coverage.rs:55-59` (`run_about_coverage`); `extensions/about/src/deps.rs:21-25` (`run_about_deps`), `:187-189` (`PullingDependency::name`, `version`); `extensions/about/src/code.rs:65-77` (`run_about_code`, `run_about_code_with`); `extensions/about/src/loc.rs:156-168` (`run_about_loc`, `run_about_loc_with`), `:219`, `:233-234` (`LocCrateRecord::regions`, `LocDocument::regions`, `crates`); `extensions/about/src/machine.rs:51` (`MachineReport::cargo`), `:87-91` (`CargoSettings::jobs`, `rustc_wrapper`, `target_dir`, `linker`, `rustflags`)

**What**: The `run_about_*` functions' docs open directly with `# Errors`, so rustdoc lifts the heading text as the summary; `run_about_units_with` opens with a parameter note. The listed public fields of serialized document types carry no doc at all, while their siblings are documented.

**Why it matters**: API-14 makes the summary sentence mandatory on public items. These are the crate's command entry points and its machine-readable `--json` contracts — the items a caller most needs described.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed run_about_* fn has a one-sentence summary paragraph before its # Errors section
- [ ] #2 Each listed public field has a doc comment
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
