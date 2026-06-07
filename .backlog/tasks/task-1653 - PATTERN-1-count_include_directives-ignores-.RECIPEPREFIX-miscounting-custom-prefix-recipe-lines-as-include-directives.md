---
id: TASK-1653
title: >-
  PATTERN-1: count_include_directives ignores .RECIPEPREFIX, miscounting
  custom-prefix recipe lines as include directives
status: Done
assignee: []
created_date: '2026-06-07 10:54'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/import_makefile_cmd.rs:173-184`

**What**: `count_include_directives` excludes recipe lines only via `!line.starts_with('\t')`, while `parse_targets` (same file, lines 196-216) additionally tracks `.RECIPEPREFIX` reassignment. Under `.RECIPEPREFIX = >`, a recipe line such as `>include extra.conf` is not tab-prefixed, so its first token matches `include` and it is counted as an include directive.

**Why it matters**: The count drives user-facing output: a spurious "N include directive(s) not followed" note on the picker, or — when no targets parse — a misleading error hint. The recipe-line-detection predicate has already diverged between the two parsers in the same module; sharing the `.RECIPEPREFIX`-aware skip (e.g. fold include counting into the `parse_targets` line loop, or extract a shared recipe-line predicate) removes both the false positive and the divergence. Same class as TASK-1492 (parser state edge case causing false-positive diagnostics).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Recipe lines under a custom .RECIPEPREFIX are not counted as include directives
- [x] #2 Recipe-line detection logic is shared between parse_targets and include counting (single predicate or single pass)
- [x] #3 Regression test: .RECIPEPREFIX = > followed by a >include recipe line yields count 0
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Extracted shared is_recipe_line predicate and observe_recipe_prefix helper used by both parse_targets and count_include_directives; the counter now tracks .RECIPEPREFIX. Regression test count_include_directives_honours_recipeprefix covers >include under .RECIPEPREFIX = > (count 0) and the empty-assignment reset.
<!-- SECTION:NOTES:END -->
