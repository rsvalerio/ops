---
id: TASK-1737
title: >-
  TEST-31: README-documented ops subcommands have no spawned-binary test; the
  pre-commit exit-code contract is untested at any level
status: Done
assignee:
  - TASK-1982
created_date: '2026-08-27 11:12'
updated_date: '2026-08-28 19:26'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/cli/tests/integration.rs
  - crates/cli/src/subcommands.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/tests/integration.rs`

**What**: `tests/integration.rs` spawns `ops` for `--version`, `--help`, `init`, dynamic-command run, `--dry-run`, `theme list`, and `about`. Every other command listed in the README command table is never run as a process:

- `ops sec` (README line 129) — plus `ops sec --dry-run`, `--skip`, `--force`
- `ops trailing-whitespace` / `tw` (line 130)
- `ops end-of-file-fixer` / `eof` (line 131)
- `ops check-json` / `ops check-yaml` (line 132)
- `ops extension list` / `ops extension show <name>` (line 125)
- `ops new-command` (lines 39, 122) and `ops import-makefile` (line 123) — at minimum their non-TTY refusal and exit code
- `ops create-review-tasks`, `ops run-before-push`

The worst of these is the **exit-code contract**, which the README states explicitly for the fixers/checkers ("non-zero when files changed (pre-commit contract)"). That mapping lives in `crates/cli/src/subcommands.rs`:

- `run_text_fixer` (line 305): `report.changed()` -> `ExitCode::FAILURE`, else `SUCCESS`
- `run_config_checker` (line 343): `report.failed()` -> `ExitCode::FAILURE`, else `SUCCESS`

Neither helper is referenced by any test in the crate (`grep -rn run_text_fixer\|run_config_checker crates/cli` finds only the definitions and the four public wrappers). There is no unit test and no integration test. A regression that inverted the branch, or dropped the `FAILURE` arm, would make every `ops tw` / `ops eof` / `ops check-json` git pre-commit hook pass silently while files were rewritten under the committer — the exact failure mode the contract exists to prevent, and the same class of bug the crate already guards against in `prompt_hook_install` (which deliberately returns FAILURE so git cannot treat an unconfigured hook as a pass).

`ops sec` is the second priority: it is the terminal step of `ops qa` per AGENTS.md, and its aggregated exit code (non-zero if any scan reports findings) is the whole point of the command, yet only `build_plan`/`run_sec_to` unit tests exist.

**Why it matters**: TEST-31 — a CLI's real interface is the binary. Argument spellings, aliases (`tw`, `eof`), stdout-vs-stderr routing, and exit codes are exactly where CLI regressions land, and unit tests over internal functions cover none of it. Here the untested surface is a documented contract that other tools (git hooks, CI gates) depend on for their own pass/fail decision, so a silent regression converts a quality gate into a no-op without any test failing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tests/integration.rs spawns 'ops trailing-whitespace' (and its 'tw' alias) in a temp dir containing a file with trailing whitespace and asserts the process exits non-zero AND the file was rewritten; a second case with a clean file asserts exit 0
- [x] #2 The same change/no-change exit-code pair is covered as spawned commands for 'ops end-of-file-fixer' (and 'eof'), 'ops check-json', and 'ops check-yaml'
- [x] #3 tests/integration.rs covers 'ops sec --dry-run' as a spawned command, asserting the plan lines and exit 0 without requiring trivy on PATH, and asserting the trivy-missing note when trivy is absent
- [x] #4 tests/integration.rs covers 'ops extension list' and 'ops extension show <name>' as spawned commands, asserting on the rendered header rather than only on success()
- [x] #5 'ops new-command' and 'ops import-makefile' are covered as spawned commands for their non-TTY refusal path: stderr names the command and the interactive-terminal requirement, and the exit code is non-zero
- [x] #6 Every new test goes through the existing ops() helper so HOME/XDG_CONFIG_HOME isolation and OPS_* clearing are inherited
<!-- AC:END -->
