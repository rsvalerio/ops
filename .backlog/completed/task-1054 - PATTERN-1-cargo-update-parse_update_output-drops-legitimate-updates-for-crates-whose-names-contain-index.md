---
id: TASK-1054
title: >-
  PATTERN-1: cargo-update parse_update_output drops legitimate updates for
  crates whose names contain 'index'
status: Done
assignee: []
created_date: '2026-05-07 21:03'
updated_date: '2026-05-08 06:35'
labels:
  - code-review
  - extensions-rust
  - cargo-update
  - PATTERN-1
  - ERR-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/cargo-update/src/lib.rs:113-116 filters `if clean.starts_with("Updating") && clean.contains("index")` to skip the `Updating crates.io index` noise line. The `contains("index")` predicate is matched anywhere in the line, so a real update for a crate whose name contains 'index' (e.g. indexer, index-map, reindex, tantivy-index) is silently dropped: 'Updating indexer v1.0.0 -> v1.0.1' matches and is skipped. parse_action_line never runs for that line, the entry is missing from CargoUpdateResult.entries, and update_count is silently low. No tracing breadcrumb (the verb-prefixed line never reaches the starts_with_known_verb branch because the early continue fires first).

Repro: include a dep whose registry name contains 'index' and run cargo update --dry-run; the resulting CargoUpdateResult will not list it as an available update.

Fix: tighten the index-noise gate to the documented exact form, e.g. `clean == "Updating crates.io index" || clean.starts_with("Updating crates.io index (")` (cargo prepends the registry name in parens for alternate registries), or assert the line contains no `->` arrow before treating it as the index-progress line. Add a regression test for crate names containing 'index'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_update_output emits an UpdateEntry for 'Updating indexer v1.0.0 -> v1.0.1'
- [x] #2 Real 'Updating crates.io index' noise lines remain filtered
- [x] #3 Regression test pins both behaviours
<!-- AC:END -->
