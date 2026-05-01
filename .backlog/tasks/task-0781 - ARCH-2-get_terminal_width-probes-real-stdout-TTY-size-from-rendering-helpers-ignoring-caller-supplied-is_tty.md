---
id: TASK-0781
title: >-
  ARCH-2: get_terminal_width probes real stdout TTY/size from rendering helpers,
  ignoring caller-supplied is_tty
status: To Do
assignee:
  - TASK-0828
created_date: '2026-05-01 05:57'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:14`

**What**: get_terminal_width() calls std::io::stdout().is_terminal() and detect_terminal_width() directly. It is invoked by cards::layout_cards_in_grid (cards.rs:158), called from units::run_about_units_with(writer, is_tty). Even when caller hands in a non-TTY writer with is_tty = false (TASK-0411 contract), layout still queries the real stdout.

**Why it matters**: Breaks writer/is_tty separation TASK-0411 established. Tests that capture output into a Vec<u8> with is_tty=false get layout sized to the developer's actual terminal, producing nondeterministic output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Thread is_tty/an explicit width down through layout_cards_in_grid (or accept Option<usize> for the width) instead of probing global stdout
- [ ] #2 Add a regression test asserting layout_cards_in_grid_with_width is the only width source touched when is_tty = false
<!-- AC:END -->
