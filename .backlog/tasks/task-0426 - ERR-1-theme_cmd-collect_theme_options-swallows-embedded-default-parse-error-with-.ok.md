---
id: TASK-0426
title: >-
  ERR-1: theme_cmd::collect_theme_options swallows embedded-default parse error
  with .ok()
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 04:41'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:20`

**What**: `parse_default_config().ok()` swallows the anyhow error from parsing the embedded default TOML. If the embedded default config ever fails to parse (regression in `default_ops_toml()`), `default_config` becomes None, every loop iteration sees `is_default = false`, and `ops theme list` prints every built-in theme labelled `(custom)` with no diagnostic surfaced to the user.

**Why it matters**: Static parse failures are infallible today, but the silent-failure shape means a refactor that breaks the embedded default would only surface as a cosmetic bug — exactly the failure mode ERR-1 ("propagate or handle, never both, and never silently swallow") targets.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On Err, log at tracing::warn! (or tracing::error!) with the parse error
- [ ] #2 Add a unit test that asserts parse_default_config().is_ok() so a regression in the embedded TOML fails CI loudly
<!-- AC:END -->
