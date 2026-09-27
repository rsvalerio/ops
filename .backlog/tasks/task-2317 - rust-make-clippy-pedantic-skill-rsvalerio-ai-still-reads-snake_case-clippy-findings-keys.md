---
id: TASK-2317
title: 'rust-make-clippy-pedantic skill (rsvalerio/ai) still reads snake_case clippy-findings keys'
status: Triage
assignee: []
created_date: '2026-09-27 15:30'
labels:
  - code-review-rust
  - consistency
  - clippy
  - json
dependencies: []
modified_files: []
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `rsvalerio/ai: skills/rust-make-clippy-pedantic/SKILL.md`, `references/extraction.md`, `references/lint-catalog.md` (external repo; the ops side is `crates/cli/src/clippy_findings_cmd.rs`)

**What**: `ops clippy-findings` now emits schema v2 with camelCase keys (`schemaVersion`, `manifestDir`, `targetKind`, `droppedOutOfTree`, `rustcWarnings`). The pedantic skill in rsvalerio/ai still documents and reads `manifest_dir`, `target_kind`, `dropped_out_of_tree`, `rustc_warnings`.

**Why it matters**: once an ops release carrying schema v2 ships, the skill reads `null` for those fields — breaking crate aggregation (`manifestDir`/`targetKind`) and the dropped/rustc counts. It should also check `schemaVersion == 2`.

**Origin**: discovered during TASK-2316 while fixing TASK-2312.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rust-make-clippy-pedantic reads the camelCase v2 keys and checks schemaVersion
<!-- AC:END -->
