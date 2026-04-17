---
id: TASK-0025
title: 'CQ-1: print_categorized_help exceeds FN-1 and mixes abstraction levels'
status: Done
assignee: []
created_date: '2026-04-14 19:13'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - FN-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cli/main.rs:236-369 (133 lines) — print_categorized_help collects built-in subcommands from clap, merges dynamic config/stack commands, sorts by category rank, formats grouped sections, and injects into clap help output. Four abstraction levels in one function: (1) clap introspection, (2) command entry collection, (3) sorting/ranking, (4) string rendering. Violates FN-1 (≤50 lines) and mixes abstraction levels. Affected crate: ops-cli.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract helpers: collect_command_entries(), sort_entries_by_category(), render_grouped_sections(). Each function ≤50 lines, operating at a single abstraction level.
<!-- AC:END -->
