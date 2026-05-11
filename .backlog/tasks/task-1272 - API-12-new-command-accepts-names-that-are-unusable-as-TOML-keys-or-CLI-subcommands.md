---
id: TASK-1272
title: >-
  API-12: new-command accepts names that are unusable as TOML keys or CLI
  subcommands
status: Done
assignee: []
created_date: '2026-05-10 23:10'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:30-37`

**What**: `run_new_command_with_tty_check` collects a command `name` via `inquire::Text` and only validates that it is non-empty after trimming. Any other string is accepted and written into `.ops.toml` as the `[commands.<name>]` table key — including names with whitespace, leading dashes, dots, slashes, or other characters that are not legal as a `clap::Subcommand` external name. Once written, the user cannot invoke the command via `ops <name>` (clap rejects/parses the arg differently) and the on-disk config can only be repaired by hand.

Concretely, names such as `build test`, `--build`, `../escape`, or an empty-after-quoting key all pass the current guard and silently produce a broken config row.

**Why it matters**: A new-command flow whose only failure mode is silently writing a config that the very next `ops <name>` invocation cannot reach is a UX bug with no recovery path inside the tool. The validation should live next to the prompt where the user can correct it interactively, not surface as an inscrutable clap error later. This is API-3-class input validation at the CLI boundary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reject names that contain ASCII whitespace, control characters, or path separators with a clear inquire-level error and re-prompt
- [x] #2 Reject names beginning with '-' (would be parsed as a flag by clap)
- [x] #3 Add unit tests covering each rejected pattern via run_new_command_with_tty_check with a stub TTY check, asserting no write to .ops.toml occurs
- [x] #4 Keep the existing empty-name check; document the accepted-name shape in the inquire help message
<!-- AC:END -->
