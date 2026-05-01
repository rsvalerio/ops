---
id: TASK-0784
title: >-
  ERR-1: layout_cards_in_grid silently uses 120-col fallback when stdout is not
  a TTY and COLUMNS is unset
status: To Do
assignee:
  - TASK-0828
created_date: '2026-05-01 05:58'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:158`

**What**: get_terminal_width returns parse_terminal_width(env::var("COLUMNS").ok().as_deref()) which silently defaults to 120 in non-TTY contexts. There is no diagnostic when this fallback fires; piped invocation produces over-wide output without any signal.

**Why it matters**: Combined with the ARCH-2 finding above, callers writing to non-TTY buffers get layout sized to a hard-coded 120 cols regardless of caller intent. The function should accept caller-supplied width so the silent default is opt-in. (Distinct from TASK-0667 which only added the TTY probe.)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Provide an explicit-width entry point used by buffer-writing call sites; reserve the env/TTY probe for direct stdout writes
- [ ] #2 Document the 120-col default in the API contract
<!-- AC:END -->
