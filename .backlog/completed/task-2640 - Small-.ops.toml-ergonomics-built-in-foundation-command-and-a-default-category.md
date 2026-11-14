---
id: TASK-2640
title: 'Small .ops.toml ergonomics: built-in foundation command and a default category'
status: Done
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-11-01 00:27'
labels:
  - feature
  - config
dependencies: []
parent_task_id: 'TASK-2645'
modified_files: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two repetitions seen in dbsec's .ops.toml:
- Every Rust repo on the foundation defines [commands.foundation] program = "ops", args = ["init", "--rust", "--check"] so qa can name it. A built-in foundation command (Rust stack) would remove that, and the ops pin of the outer process would be used instead of whatever ops is on PATH.
- category = "Code Quality" is repeated on ~15 commands. A default, e.g. a [commands] level or per-section default category, or inheriting category from the composite a command is listed in, would remove the repetition.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops foundation works with no [commands.foundation] entry on a Rust stack
- [x] #2 A decision recorded on default categories

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1: ops foundation is a built-in running the same check as ops init --rust --check, re-invoking the current ops binary. It resolves in composites with no [commands.foundation]; checked with ops explain on a scratch crate. A leftover [commands.foundation] gets the existing shadowed warning; docs/foundation.md says to delete it. On this repo it exits 1 for rustfmt.toml drift, same as init --rust --check (TASK-2648).

AC #2 decision: no default category, recorded in docs/configuration.md under "Command categories". [commands] keys are command names, so a default needs a reserved name or a new section with defined layer scope, for a help heading only. Inheriting from a composite is ambiguous because a command can sit in several, as clippy does in verify and verify-fix. Per-command category stays, and clone already carries it.
<!-- SECTION:NOTES:END -->
