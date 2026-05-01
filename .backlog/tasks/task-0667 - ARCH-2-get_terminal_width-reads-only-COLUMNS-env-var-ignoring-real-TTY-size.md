---
id: TASK-0667
title: 'ARCH-2: get_terminal_width reads only COLUMNS env var, ignoring real TTY size'
status: Done
assignee:
  - TASK-0742
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 20:04'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:8-17`

**What**: Reads `COLUMNS` env var only; never queries the actual TTY size via `terminal_size`/`tput`. When `COLUMNS` is unset (the default in many shells until the user resizes the terminal once), it falls back to a hard-coded 120, which over- or underflows the `cards` grid layout on real terminals.

**Why it matters**: Card grid in `cards::layout_cards_in_grid_with_width` chooses 1/2/3 columns from this value; getting it wrong makes the rendered output wrap badly on small terminals or under-utilise wide ones.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Have get_terminal_width consult an actual TTY-size source (e.g. the existing terminal_size crate already in the workspace, or ops_core::output) and fall back to COLUMNS only when stdout is not a tty
- [ ] #2 Keep the pure parse_terminal_width for tests
<!-- AC:END -->
