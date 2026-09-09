---
id: TASK-2075
title: 'READ-13: AboutAction Code/Crates variants leak internal rationale into ops about --help'
status: To Do
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - crates/cli/src/args.rs
priority: low
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:493` (AboutAction::Code) and `crates/cli/src/args.rs:516` (AboutAction::Crates)

**What**: Both variants carry two concatenated doc comments — an internal-rationale paragraph plus the one-line user-facing summary. clap folds every `///` line on a variant into the subcommand help, so `ops about --help` (long help) prints developer reasoning such as "Gating the variant under the duckdb feature keeps the CLI surface honest - without DuckDB the binary has no way to compute the stats..." (Code) and "the alias keeps the Go-idiomatic name working without duplicating dispatch" (Crates).

The codebase already knows this is wrong: the neighbouring `Loc` variant (args.rs:502-513) carries the same rationale as a plain `//` comment with an explicit note that clap folds `///` into user-facing help and that "the neighbouring variants' internal reasoning is currently printed to users by ops about --help". `Loc` was fixed; `Code` and `Crates` were not.

**Why it matters**: Internal design notes in the help screen dilute the user-facing surface and drift from the established pattern in the same enum. It is a self-acknowledged inconsistency: the comment documenting the fix names these variants as still broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutAction::Code rationale paragraph converted to a plain // comment, keeping the one-line user-facing doc 'Display code statistics (lines of code, languages)'
- [ ] #2 AboutAction::Crates rationale paragraph converted to a plain // comment, keeping the one-line user-facing doc
- [ ] #3 a test asserts ops about --help output contains no internal-rationale phrases (e.g. 'Gating the variant', 'duplicating dispatch')
<!-- AC:END -->
