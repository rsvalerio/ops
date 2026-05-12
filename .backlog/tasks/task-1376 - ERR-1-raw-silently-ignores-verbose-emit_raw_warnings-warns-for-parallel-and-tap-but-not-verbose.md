---
id: TASK-1376
title: >-
  ERR-1: --raw silently ignores --verbose; emit_raw_warnings warns for parallel
  and tap but not verbose
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 21:46'
updated_date: '2026-05-12 23:34'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:172-176` (`run_commands` dispatch) and `crates/cli/src/run_cmd.rs:238-253` (`emit_raw_warnings`)

**What**: When `--raw` is selected, `run_commands` calls `run_commands_raw(&runner, plan, tap.as_ref())` which never reads the `verbose` field of `RunOptions`. The companion warning helper `emit_raw_warnings` emits a `tracing::warn!` for `--raw --parallel` and `--raw --tap` but not for `--raw -v`/`--raw --verbose`. The user invokes `ops <cmd> --raw -v` expecting verbose stderr-tail handling and gets a silent override (`-v` is documented as "Show full stderr output on failure (overrides stderr_tail_lines config)" in args.rs:28-30, but raw mode inherits child stdio directly and has no stderr tail to enhance).

The dry-run path was retrofitted with the same contract in TASK-1234 (`dry_run_overrides_messages` emits warnings for `--raw` and `--tap`-under-`--dry-run`). The raw path is the inverse asymmetry — it warns about `--tap` but not `--verbose`.

**Why it matters**: Silent overrides break user trust in the flag system: a user can spend minutes wondering why `-v` had no effect under `--raw`. The fix is a one-line addition to `emit_raw_warnings` mirroring the existing `has_tap` branch. The bigger structural question — whether `--raw` should reject conflicting flags via `conflicts_with` (like `--raw` and `--tap` already do via clap) or merely warn — is worth raising; clap-level rejection prevents the override entirely, while a warning preserves the freedom to set globals in shell aliases.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either: --raw and --verbose are declared mutually exclusive at the clap layer (conflicts_with), OR emit_raw_warnings emits a tracing::warn! when verbose is set under raw, with a unit test asserting the message
- [ ] #2 If the warning path is chosen, dry_run_overrides_messages picks up a symmetric --verbose-under-dry-run branch (the dry-run path also never honours -v's stderr-tail behaviour, since dry-run never executes)
- [ ] #3 A unit test asserts the new warning fires, mirroring the existing emit_raw_warnings parallel/tap test pattern
<!-- AC:END -->
