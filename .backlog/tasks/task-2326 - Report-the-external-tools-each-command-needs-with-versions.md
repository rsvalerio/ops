---
id: TASK-2326
title: 'Report the external tools each command needs, with versions'
status: Triage
assignee: []
created_date: '2026-09-28 10:59'
labels:
  - ci
  - ops-alignment
dependencies: []
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
- [ ] #1 `ops explain <cmd> --json` (or a tools subcommand) lists required binaries for the resolved plan
<!-- AC:END -->
