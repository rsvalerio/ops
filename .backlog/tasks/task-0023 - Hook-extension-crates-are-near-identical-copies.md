---
id: TASK-0023
title: Hook extension crates are near-identical copies
status: Triage
assignee: []
created_date: '2026-04-11 18:30:00'
labels:
  - rust-code-duplication
  - CD
  - DUP-1
  - DUP-2
  - medium
  - ext-run-before-commit
  - ext-run-before-push
dependencies: []
---

## Description

**Location**: `extensions/run-before-commit/src/lib.rs:28-172`, `extensions/run-before-push/src/lib.rs:28-157`
**Anchor**: `fn find_git_dir`, `fn install_hook`, `fn ensure_config_command`, `fn should_skip`
**Impact**: The two hook extension crates share ~115 lines of production code that are structurally identical, differing only in hook name strings ("pre-commit" vs "pre-push", "run-before-commit" vs "run-before-push"). Four functions are near-copies:

- `find_git_dir` — 12 identical lines (DUP-1: verbatim copy)
- `should_skip` — 3 identical lines differing only in the env var name constant
- `install_hook` — ~40 lines with the same flow (create hooks dir → check existing hook → compare content → detect legacy patterns → write + chmod), differing only in hook file name and legacy detection strings
- `ensure_config_command` — ~60 lines with the same TOML manipulation flow (read/create doc → check existing key → insert table with commands/fail_fast/help), differing only in the command key and help string

**Notes**:
TASK-0004 (Done) addressed the CLI-side install orchestration in `crates/cli/src/` by extracting `hook_shared.rs` with `HookOps`. That same pattern could be extended to the extension library layer. A shared crate or module (e.g., `extensions/hook-shared/`) could provide:

1. A generic `find_git_dir` (only one copy needed — identical in both)
2. A parameterized `install_hook(git_dir, hook_name, hook_script, legacy_patterns, w)` that takes the hook-specific strings as arguments
3. A parameterized `ensure_config_command(config_dir, command_name, help_text, selected, w)` similarly parameterized

Each extension crate would then reduce to: constants + extension metadata + a call to the shared functions. Test code is also duplicated but defers to rust-test-quality per DUP-10.

DUP-1: `find_git_dir` is a verbatim 12-line copy. DUP-2: 4 functions with similar structure differing only in string literals.
