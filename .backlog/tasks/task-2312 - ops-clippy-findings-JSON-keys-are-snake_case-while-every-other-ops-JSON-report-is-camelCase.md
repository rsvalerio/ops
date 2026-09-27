---
id: TASK-2312
title: 'ops clippy-findings: JSON keys are snake_case while every other ops JSON report is camelCase'
status: To Do
assignee: []
created_date: '2026-09-27 15:20'
updated_date: '2026-09-27 15:25'
labels:
  - cli
  - clippy
  - json
  - consistency
dependencies: []
parent_task_id: 'TASK-2316'
modified_files:
  - crates/cli/src/clippy_findings_cmd.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops clippy-findings` emits `schema_version`, `dropped_out_of_tree`, `rustc_warnings`, `manifest_dir`, `target_kind`. Every other versioned ops JSON report uses camelCase: `schemaVersion` in `about machine/crates/dependencies`, `explain`, `backlog task view`, `wave overlap`, and `manifestDir` in `about crates`. The same concept appears as `manifestDir` in one report and `manifest_dir` in another.

**Why it matters**: consumers (the `rust-make-clippy-pedantic` skill in rsvalerio/ai reads both `ops about crates` and `ops clippy-findings`) have to remember a different key style for one command. A consumer writing `.schemaVersion` against this report silently gets `null`.

**Sketch**: switch the report to camelCase with `#[serde(rename_all = "camelCase")]`, and bump `SCHEMA_VERSION` to 2, since it is an incompatible row change by the file's own rule. The only known consumer is the pedantic skill, which will be updated alongside.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 clippy-findings JSON uses camelCase keys, consistent with the other ops JSON reports
- [ ] #2 schemaVersion is bumped, and a test pins the new key names
<!-- AC:END -->
