---
id: TASK-2324
title: 'Give ops deps a non-mutating, CI-safe check mode'
status: Triage
assignee: []
created_date: '2026-09-28 10:59'
labels:
  - ci
  - ops-alignment
dependencies: []
modified_files: []
priority: medium
ordinal: 1000
dedup_key: 'ops-align:ops-deps-check'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `qa` runs `ops deps`, which runs `cargo upgrade` (cargo-edit) as well as cargo-deny and machete. In CI a gate must never edit Cargo.toml, and should not need cargo-edit just to run `cargo deny check` (what forge rust-ci's deps job does today). Note: ops deps is under active work (TASK-2321).

**Why it matters**: needed for forge TASK-0026 to replace rust-ci's deps job.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A check mode that reports (advisories, licenses, bans, unused) and fails without editing files or requiring cargo-edit
<!-- AC:END -->
