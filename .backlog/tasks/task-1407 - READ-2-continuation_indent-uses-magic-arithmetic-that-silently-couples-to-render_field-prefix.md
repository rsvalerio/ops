---
id: TASK-1407
title: >-
  READ-2: continuation_indent uses magic arithmetic that silently couples to
  render_field prefix
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:10'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:203-205`

**What**: `continuation_indent` returns `" ".repeat(2 + 2 + 1 + (max_key_len + 2) + 1)` — an opaque arithmetic expression whose meaning lives in a doc comment, not in named constants. The same column structure is implicitly duplicated in `render_field`'s `format!("  {} {} {}", emoji, padded_key, ...)`.

**Why it matters**: Drift in either site silently misaligns continuation lines. Extract named constants (`LEADING`, `EMOJI_COLS`, `SEP`) or compute the indent from the prefix length actually rendered by `render_field`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Continuation indent is derived from named constants or from the rendered prefix length
<!-- AC:END -->
