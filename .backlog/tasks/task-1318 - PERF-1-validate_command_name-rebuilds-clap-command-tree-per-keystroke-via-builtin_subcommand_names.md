---
id: TASK-1318
title: >-
  PERF-1: validate_command_name rebuilds clap command tree per keystroke via
  builtin_subcommand_names()
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-11 20:34'
updated_date: '2026-05-17 09:23'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:33-43, 99-107`

**What**: `validate_command_name` is wired as the live validator on the `inquire::Text` prompt for the new command name (lines 33-39). On every keystroke, inquire calls the validator, which calls `validate_command_name(input.trim())`. That function in turn calls `builtin_subcommand_names()` at line 86, and `builtin_subcommand_names()` (line 99) unconditionally invokes `clap::CommandFactory::command()`, walks `get_subcommands()`, and allocates a fresh `Vec<String>` of name clones on every call. `clap::Command` construction is non-trivial (it builds the full subcommand tree with help text, args, value parsers, etc.), so we pay that cost — plus an allocation per built-in name — for every character the operator types into the prompt.

The fix is the same pattern `theme_cmd::BUILTIN_THEME_NAMES` already uses (lines 31-51 of theme_cmd.rs): cache the built-in subcommand names in a `OnceLock<HashSet<&'static str>>` (or `OnceLock<Vec<&'static str>>`) so the clap walk runs once for the process, not once per keystroke.

**Why it matters**: The user-visible cost is small per keystroke but compounds quickly during interactive editing — a 40-character paste triggers 40 full clap command-tree rebuilds. More importantly, the project already has the cached-builtin pattern in this exact crate (theme_cmd.rs), and the un-cached version here was added in TASK-1296 without picking up that idiom. Filing this so the next maintainer touching the validator brings it in line with the project's own established cache pattern.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 builtin_subcommand_names() (or its result) is cached for the process lifetime, so a 40-character paste into the inquire prompt does not trigger 40 calls to clap::CommandFactory::command()
- [x] #2 The cache pattern matches the existing theme_cmd::BUILTIN_THEME_NAMES OnceLock idiom (same crate)
- [x] #3 All existing tests still pass and validate_command_name continues to reject every clap-registered built-in
<!-- AC:END -->
