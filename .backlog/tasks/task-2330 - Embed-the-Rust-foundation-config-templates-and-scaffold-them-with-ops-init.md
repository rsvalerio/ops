---
id: TASK-2330
title: 'Embed the Rust foundation config templates and scaffold them with ops init'
status: Triage
assignee: []
created_date: '2026-09-28 13:52'
labels:
  - ci
  - ops-alignment
dependencies: []
modified_files: []
priority: medium
ordinal: 1000
dedup_key: 'ops-align:ops-scaffold'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: the shared Rust config files (clippy.toml, deny.toml, rustfmt.toml, .config/nextest.toml, the [workspace.lints] policy, the test profile) have no single source: forge ships config/* that dbsec vendors via a private forge-sync script, and the ai skills rust-make-clippy-pedantic (apply-config.md) and rust-make-build-fast (apply-templates.md) carry their own divergent copies.

**Decision** (owner, 2026-09-28, recorded on forge TASK-0025): ops is the single source. ops embeds the templates, a scaffold command (e.g. `ops init --stack rust`) writes them into a repo, and a check reports drift from the running ops version's copy. forge's config/ and the ai skills' templates then reference ops.

**Why it matters**: one policy for every Rust repo, updated with ops releases, no vendored copies to sync.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops embeds templates for clippy.toml, deny.toml, rustfmt.toml, .config/nextest.toml and the [workspace.lints] policy, reconciled from forge config/* and the ai skills' templates
- [ ] #2 A scaffold command writes them into a Rust repo without overwriting local edits unless asked
- [ ] #3 A check reports drift between a repo's files and the running ops version's templates
<!-- AC:END -->
