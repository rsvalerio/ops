---
id: TASK-1220
title: 'PERF-3: render_field aligns keys by byte length instead of display width'
status: Done
assignee:
  - TASK-1271
created_date: '2026-05-08 12:56'
updated_date: '2026-05-09 07:50'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:210-216`

**What**: `render_field` uses `format!("...{:<width$}...", key, width = max_key_len + 2)` where `max_key_len` comes from `(k, _) | k.len()` (byte length). Multi-byte labels misalign by one column per non-ASCII char. TASK-1187 covers `format_language_breakdown`; this is a separate render site.

**Why it matters**: About-card key column drifts under non-ASCII labels — cosmetic but breaks alignment guarantees the theme contract advertises. Same root cause as the codebase-breakdown task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace k.len() with crate::output::display_width(k) in max-len reduction
- [ ] #2 Continuation indent uses display width of max_key_len
- [ ] #3 Add regression test with a multi-byte key
<!-- AC:END -->
