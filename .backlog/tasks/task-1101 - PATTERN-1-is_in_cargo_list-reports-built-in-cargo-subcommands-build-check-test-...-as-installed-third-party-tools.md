---
id: TASK-1101
title: >-
  PATTERN-1: is_in_cargo_list reports built-in cargo subcommands (build, check,
  test, ...) as installed third-party tools
status: Done
assignee: []
created_date: '2026-05-07 21:33'
updated_date: '2026-05-08 06:18'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:135-149`

**What**: `cargo --list` enumerates every subcommand cargo knows about, including built-ins (`build`, `check`, `clippy`, `test`, `run`, etc.) alongside external `cargo-*` binaries. When a `tools.toml` entry shares a name with a built-in, `check_tool_status_with` always returns `Installed` — even though `cargo install cargo-build` was never run and `install_tool` would silently succeed-without-installing because `is_installed == true`.

**Why it matters**: The collision is rare in practice but the failure mode is silent — user thinks the tool is present, it isn't.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 is_in_cargo_list either filters known-builtin names against an allow/deny list, or matches only on lines whose token has a cargo--shaped install hint visible to the operator
- [ ] #2 Test pins that is_in_cargo_list("build", "    build\n    cargo-foo\n") returns false (or the chosen tightened semantic)
- [ ] #3 Doc comment notes the historical false-positive against built-ins
<!-- AC:END -->
