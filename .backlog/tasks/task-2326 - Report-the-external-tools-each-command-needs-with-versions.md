---
id: TASK-2326
title: 'Report the external tools each command needs, with versions'
status: Done
assignee: []
created_date: '2026-09-28 10:59'
updated_date: '2026-09-28 15:19'
labels:
  - ci
  - ops-alignment
dependencies: []
parent_task_id: 'TASK-2333'
modified_files: []
priority: low
ordinal: 1000
dedup_key: 'ops-align:ops-tools'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `deps` needs cargo-edit/cargo-deny/machete, `next` needs nextest, `sec` needs Trivy; nothing machine-readable says so, so every CI hand-maintains an install list (forge has four methods).

**Why it matters**: lets forge TASK-0026/forge TASK-0030 derive installs from ops instead of duplicating them.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ops explain <cmd> --json` (or a tools subcommand) lists required binaries for the resolved plan

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
ops explain --json now carries a top-level `tools` list ({name, optional, install, requiredBy}) and a per-step `tools` array, derived statically in crates/cli/src/run_cmd/tools.rs (cargo plugins, sec -> trivy, deps -> ops_deps::external_tools()). Versions (title) are not reported: explain never spawns a process, so installed/minimum versions are split into a Triage follow-up.
<!-- SECTION:NOTES:END -->
