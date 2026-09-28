---
id: TASK-2321
title: 'ops deps: report unused dependencies via cargo-machete'
status: To Do
assignee: []
created_date: '2026-09-27 20:57'
updated_date: '2026-09-28 08:57'
labels:
  - feature
  - deps
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/types.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/parse/mod.rs
priority: medium
ordinal: 162000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Context**: `ops deps` (extensions-rust/deps) combines `cargo upgrade --dry-run` and `cargo deny check` into six checks: compatible upgrades, breaking upgrades, advisories, licenses, duplicate crates, sources. Neither tool looks at whether a declared dependency is used, so a dependency dropped from the code stays in Cargo.toml, is compiled on every build and still passes the gate. Found while cutting event0's build cost (event0 TASK-0668.06), where the unused-deps check had to be a separate one-off.

**What**: add a seventh check, **Unused Dependencies**, backed by `cargo machete` and following the crate's existing pattern: a `CargoTool` entry (`subcommand: "machete"`, `install_crate: "cargo-machete"`), a `run_cargo_machete` / `interpret_machete_*` pair with the same format-drift guards as upgrade and deny (fail-closed on output it does not recognise), a field on `DepsReport`, and a row in `build_report`.

**Decisions this needs**:
- **Required or optional tool**: `REQUIRED_CARGO_TOOLS` makes a missing tool fail `ops deps`, and every downstream `ops qa` runs `deps` first under fail_fast (event0's AGENTS.md documents this). Making machete required breaks every consumer's qa until they install it. Consider an optional tool that renders the row as skipped with the install hint.
- **Warning or failure**: machete is heuristic (it greps sources), so false positives on macro-only, build-script and feature-gated uses are common. Unused deps probably belong at the Duplicate Crates level (warning), not at advisories (failure). Suppressions come from `[package.metadata.cargo-machete] ignored = [...]`, which the row should respect rather than re-implement.
- **Parsing**: check whether the installed cargo-machete has machine-readable output before parsing its text; pin the parser against the exit code as well (machete exits 1 when it finds something, which the interpreter must not read as the tool failing).
- **`--with-metadata`**: more accurate (uses cargo metadata for renamed crates) but slower; measure on a real workspace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops deps shows an Unused Dependencies row listing each unused dependency with its crate and manifest path
- [x] #2 A missing cargo-machete is handled per the recorded required/optional decision, with the install hint, and the decision is documented in the crate docs
- [x] #3 Unrecognised machete output or exit codes fail closed, covered by tests like the existing parse/deny and parse/upgrade ones
- [x] #4 Dependencies listed in [package.metadata.cargo-machete] ignored are not reported
- [x] #5 ops deps' description, README and ops about text name the new check

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (cargo-machete 0.9.2):
- Optional tool: CARGO_MACHETE is outside REQUIRED_CARGO_TOOLS; a failing probe yields UnusedDepsResult::NotInstalled and the row renders Skipped with the install hint. A probe timeout or spawn error still fails, so a stuck probe cannot quietly skip the check.
- Warning, never failure: has_issues ignores unused deps; the row is at most ReportStatus::Warning. Ignore lists come from [package.metadata.cargo-machete], which machete applies itself.
- Parsing: machete has no machine-readable output, so the parser reads stdout text and rejects any line it does not recognise. Exit 0 must match the clean summary and exit 1 must match a non-empty listing. Exit 2, a signal or any other code fails. A 'error when handling <manifest>' line on stderr fails even at exit 0, because machete skips broken manifests and still exits 0.
- No --with-metadata: 2.3s vs 0.2s on this workspace, more false positives (dev-deps), and it may rewrite Cargo.lock.
- CARGO_PKG_NAME is removed from machete's env: machete only thinks it runs as 'cargo machete' when that var is unset, so under cargo run or tests it misparsed its args and exited 2.
<!-- SECTION:NOTES:END -->
