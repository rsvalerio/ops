---
id: TASK-1518
title: >-
  FN-1: deps parse_upgrade_table_inner spans 72 lines combining header /
  separator / body / diagnostics state
status: To Do
assignee:
  - TASK-1645
created_date: '2026-05-19 07:27'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:143-214` (`parse_upgrade_table_inner`)

**What**: `parse_upgrade_table_inner` is 72 lines. It hand-rolls a multi-state line walker tracking four pieces of state (`columns`, `saw_separator`, `saw_recognised_header`, `body_lines`) and ends with a trailing tracing::warn + tuple build. Inside the for-loop it juggles:

- empty-line skip,
- header-line detection with the `saw_recognised_header` re-arm guard (TASK-1203),
- separator-line detection,
- body-line counting + `parse_upgrade_row` push,

each as a flat conditional. The function reads as the parser's main switchboard, but every new schema-drift signal added by future tasks lands inside this single body. Splitting line classification into a small `enum UpgradeLine { Header, Separator, Body }` produced by a private `classify_upgrade_line` helper would let the state-machine body collapse into a match that's easy to extend and test in isolation.

This is also adjacent to TASK-1492 (PATTERN-1: body_lines counter pre-header preamble): both bug classes point at the same overloaded function body.

**Why it matters**: FN-1 (function > 50 lines). The function is the integration seam for every cargo-edit drift fix already filed; future drift bugs slot into the same overloaded body unless the classification step is named.

<!-- scan confidence: function length verified — 72 lines per `awk '/^fn parse_upgrade_table_inner/,/^}/' | wc -l` -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_upgrade_table_inner reduced below 50 lines by extracting line classification into a named helper returning a typed enum (e.g. UpgradeLine::{Header, Separator, Body})
- [ ] #2 All existing parse_upgrade_table_* and interpret_upgrade_output_* tests pass without behavioural change
- [ ] #3 The new helper has at least one direct unit test covering each classification arm
<!-- AC:END -->
