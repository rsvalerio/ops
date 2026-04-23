---
id: TASK-0128
title: 'READ-10: --raw silently drops --tap FILE instead of warning'
status: Done
assignee: []
created_date: '2026-04-21 21:28'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:78-95, 144-168`

**What**: The CLI accepts `--raw` and `--tap FILE` together, but `--tap` is silently ignored under `--raw`:

- `run_command` (line 144) receives `tap: Option<PathBuf>` and forwards it to `run_command_cli` only on the non-raw branch (line 160); the raw branch calls `run_command_raw` (line 158), which does not accept `tap` at all (line 170).
- `run_commands` (line 60) accepts `tap` and passes it into `ProgressDisplay::new` only on the non-raw path (line 101). The raw branch (lines 78-95) never consults `tap`.

Raw mode inherits child stdio directly, so there is no stream to tap, but the user gets no feedback — an empty file may be created elsewhere, or the flag is simply dropped.

**Why it matters**: Same class of silent-ignore regression as TASK-0125: a CLI flag has no observable effect under `--raw`, so scripts and hooks that combine the two produce surprising results without a diagnostic. Parity with the existing `parallel`-ignored warning keeps the `--raw` contract predictable.

<!-- scan confidence: candidates to inspect -->
- `crates/cli/src/run_cmd.rs:78-95` (run_commands raw branch drops tap)
- `crates/cli/src/run_cmd.rs:144-168` (run_command drops tap when raw=true)
- `crates/cli/src/run_cmd.rs:170-177` (run_command_raw has no tap param)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When `--raw` is combined with `--tap FILE`, a `tracing::warn!` is emitted explaining that `--tap` is ignored because raw mode inherits stdio
- [ ] #2 Warning fires in both code paths (single-command via `run_command` and multi-command via `run_commands`)
- [ ] #3 Test covers `ops --raw --tap some.txt build` and asserts the warning is emitted and no tap file is created
<!-- AC:END -->
