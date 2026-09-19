---
id: TASK-2274
title: 'Let [extend.<name>] override help so an extended command describes what it runs'
status: Triage
assignee: []
created_date: '2026-09-19 11:23'
labels:
  - feature
  - config
dependencies: []
modified_files:
  - crates/core/src/config/extend.rs
  - README.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After `[extend.verify] commands = ["doc-default", "fuzz-fmt", "fuzz-check", "next"]` (dbsec, ops 0.60.0), `ops --help` still prints the stack default's help: "Run fmt, trailing-whitespace, end-of-file-fixer, then clippy, build, check-json, check-yaml, doc in parallel". The command runs 14 steps; the help names 8. `[extend]` accepts only `commands`/`args`, and a stack-default name cannot be redefined by `clone`, so there is no way to fix the text short of restating the whole composite -- which is what `[extend]` exists to avoid.

Two options, not exclusive:
1. `[extend.<name>] help = "..."` replaces the help (and `category`, for symmetry).
2. When a composite is extended and no help override is given, derive the help from the materialized list, or append the extras ("..., doc in parallel; then doc-default, fuzz-fmt, fuzz-check, next") so the default can never silently understate the plan.

Option 2 matters more: a stale help is the default outcome today, and nobody notices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `[extend.<name>] help = "..."` overrides the target's help; given across layers, the last layer wins
- [ ] #2 An extended composite with no help override shows help that names the appended commands
- [ ] #3 `ops --help` and `ops --dry-run` agree on what an extended command runs
- [ ] #4 README "Extending existing commands" documents the help rule
<!-- AC:END -->
