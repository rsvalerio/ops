---
id: TASK-1652
title: >-
  FN-1: run_import_makefile_with_tty_check spans ~95 lines mixing resolution,
  parsing, prompting, and writing
status: Done
assignee: []
created_date: '2026-06-07 10:53'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/import_makefile_cmd.rs:37-132`

**What**: `run_import_makefile_with_tty_check` is ~95 lines orchestrating six concerns inline: TTY gate, Makefile resolution/read, parse + include-note emission, partition + skip notes, the inquire MultiSelect prompt with cancel handling, and selected-target resolution + config write.

**Why it matters**: FN-1 (≤50 lines, single abstraction level). The prompt block (option building, preselect, cancel/error match, mapping selections back to `MakeTarget`s, lines 81-127) is a self-contained unit that would extract cleanly into e.g. `prompt_target_selection(&importable) -> anyhow::Result<Option<Vec<&MakeTarget>>>`, leaving the orchestrator readable top-to-bottom and making the cancel path unit-testable without a TTY.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_import_makefile_with_tty_check is ≤50 lines operating at one abstraction level
- [x] #2 Prompt/selection logic extracted into a named helper; existing tests still pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Extracted load_importable_targets (resolve/read/parse + include & skip notes) and prompt_target_selection (MultiSelect + cancel handling + mapping back to MakeTargets, None = cancelled). Orchestrator is now 32 lines at one abstraction level; all 274 ops tests pass.
<!-- SECTION:NOTES:END -->
