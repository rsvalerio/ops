---
id: TASK-1377
title: >-
  PATTERN-1: is_toplevel_help misclassifies ops --tap path --help as subcommand
  help
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-12 21:51'
updated_date: '2026-05-17 09:44'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:9-29`

**What**: `is_toplevel_help` decides whether the user asked for top-level help by skipping argv[0] and treating any non-flag positional as a subcommand. Global flags that take a value as a **separate** argv entry (e.g. `--tap <FILE>`) make the file path look like a positional. So `ops --tap out.log --help` falls into the "positional appeared — not top-level help" branch and returns false, even though no subcommand is present and the user clearly wants top-level help. `ops --tap=out.log --help` works because the value is folded into the same argv entry.

The args definitions in `args.rs:33-34` declare `--tap` as `Option<PathBuf>` (global), so the long form `--tap path` is canonical clap syntax; the preprocessor does not collapse it.

Repro: `ops --tap /tmp/x --help` falls through to `cmd.get_matches_from(...)` and clap then prints subcommand-style help for the catch-all `External` arm, not the categorized top-level help.

**Why it matters**: An undocumented inconsistency between `--tap=path --help` and `--tap path --help`. Top-level help is the entry point new users hit; this silently degrades that path when any global value-taking flag is in play.

**Suggested fix**: extend the scan to consume one extra argv slot when the current arg is a known value-taking global flag (`--tap`, or any short alias) before evaluating the next entry. Alternatively, mark `--tap` `value_delimiter('=')`-required, but that breaks clap convention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 is_toplevel_help returns true for ops --tap <path> --help (space-separated value form)
- [x] #2 Existing positional-subcommand cases (ops build -h) still return false
- [x] #3 Unit test pinning the new behaviour added to help.rs::tests
<!-- AC:END -->
