---
id: TASK-1413
title: >-
  ERR-1: Stack::default_commands silently degrades to empty map on embedded-TOML
  parse failure
status: Done
assignee:
  - TASK-1451
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 19:23'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/mod.rs:114-123`

**What**: When `toml::from_str(toml)` fails on an `include_str!`-embedded `.default.<stack>.ops.toml`, `default_commands` logs `tracing::warn!` and returns `IndexMap::new()`. The fallback is silent at user level: `ops init --commands` produces a stack section with zero commands and no `ui::warn` is emitted. The docstring claims a CI gate makes this unreachable, but the runtime code still chooses the silent-empty path.

**Why it matters**: If the CI gate ever lapses or a release ships a broken default TOML, users get an empty `ops init` scaffold with only a structured log line filtered by default `OPS_LOG_LEVEL=info`. Either panic (matching the CI-gate-is-the-contract argument) or route through `crate::ui::warn` so the failure is visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 emit crate::ui::warn in addition to tracing when embedded default TOML fails to parse
- [x] #2 include stack name and parse error in the user-facing message
- [x] #3 test asserts that ui::warn fires when a synthetic parse failure is injected via a test-only seam, or document the panic policy
<!-- AC:END -->
