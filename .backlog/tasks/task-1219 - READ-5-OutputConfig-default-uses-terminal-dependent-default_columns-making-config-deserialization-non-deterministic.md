---
id: TASK-1219
title: >-
  READ-5: OutputConfig::default uses terminal-dependent default_columns making
  config deserialization non-deterministic
status: To Do
assignee:
  - TASK-1271
created_date: '2026-05-08 12:56'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:297-313`

**What**: `OutputConfig`'s `columns` field defaults via `default_columns()` which probes `terminal_size::terminal_size()`. The same .ops.toml deserialised in a 200-col terminal vs an 80-col CI runner produces different Config values (and different round-trip serialisations); the `SERIALIZATION_DEFAULT_COLUMNS` skip predicate only matches the 80 sentinel.

**Why it matters**: Tests comparing Config snapshots flake under different TTY sizes; round-tripping a written config silently changes `columns`. Defaults driven by deserialisation should be deterministic; runtime terminal-aware width belongs at the render site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Make the serde default a fixed sentinel meaning auto
- [ ] #2 Resolve terminal-aware width at render time via detect_terminal_width
- [ ] #3 is_default_columns matches the new sentinel and existing tests pass
<!-- AC:END -->
