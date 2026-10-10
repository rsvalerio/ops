---
id: TASK-2604
title: 'API-14: many public struct fields in ops-about carry no doc summary'
status: To Do
assignee: []
created_date: '2026-10-10 20:49'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/identity.rs
  - extensions/about/src/machine.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/units.rs
  - extensions/about/src/deps.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/about:crate'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/identity.rs:26` (worst instance; full list below)

**What**: Public structs have item docs but many public fields have none, so rustdoc renders bare field names. Two prior API-14 waves (TASK-2071, TASK-2400) fixed item-level docs; fields were left behind. Candidates:

- `src/identity.rs` — `ParsedManifest`: all 17 fields undocumented (name, version, description, license, authors, homepage, repository, stack_label, stack_detail, module_label, module_count, loc, file_count, msrv, dependency_count, coverage_percent, languages)
- `src/machine.rs:64` — `BuildProcess.pid`, `BuildProcess.name`; `machine.rs:72` — `Setting.value`, `Setting.source`; `machine.rs:107` — `IncrementalProfiles.dev`, `.release`; `machine.rs:115` — `FsReport.path`, `.fs_type`, `.tmpfs`, `.total_bytes`, `.available_bytes`; `machine.rs:131` — `ConfigLayer.table`; `machine.rs:277` — `WorkspaceRoot.path`, `.manifest`
- `src/loc.rs:52` — `RustLocPage.regions`, `.files`; `loc.rs:220` — `LocRegionRecord`: all 7 fields; `loc.rs:234` — `LocCrateRecord.name`; `loc.rs:246` — `LocDocument.schema_version`, `.kind`
- `src/units.rs:178` — `UnitsDocument.schema_version`, `.kind`, `.crates`
- `src/deps.rs:63` — `DependencyRecord.name`; `deps.rs:71` — `UnitDepsRecord.name`, `.dependencies`; `deps.rs:79` — `DepsDocument.schema_version`, `.kind`, `.units`; `deps.rs:158` — `DuplicateReport.crates`; `deps.rs:164` — `DuplicateCrate.name`, `.versions`; `deps.rs:176` — `OlderVersion.version`; `deps.rs:196` — `DuplicatesDocument.schema_version`, `.kind`, `.crates`

Neighboring types (`MachineReport`, `CargoSettings`, `UnitRecord`, `AboutOptions`) document every field, so the crate's own convention supports the fix.

**Why it matters**: API-14: the summary is mandatory on public items; fields are public items and the first thing a consumer of the `--json` documents reads. Undocumented `schema_version`/`kind` fields on every JSON envelope especially leave the versioning contract implicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public field of the listed structs has a doc summary line
<!-- AC:END -->
