---
id: TASK-0153
title: >-
  ERR-5: toml_edit parse-error silently discarded, overwriting user config in 3
  CLI handlers
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 14:14'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/cli/src/theme_cmd.rs:196-198` (update_toml_theme)
- `crates/cli/src/about_cmd.rs:72-74` (save_about_fields)
- `crates/cli/src/new_command_cmd.rs:67-69` (append_command_to_config)

**What**: Each call to `content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|_| toml_edit::DocumentMut::new())` silently swallows the parse error when the existing `.ops.toml` is malformed, then `std::fs::write` persists a near-empty replacement — destroying the user's previous config. Same anti-pattern as TASK-0148 (hook-common) but in three additional sites that TASK-0148 does not cover.

**Why it matters**: Silent data loss on any existing `.ops.toml` with a TOML syntax error. Users running `ops theme select`, `ops about setup`, or `ops new-command` against a broken config will have the remainder of their config replaced without warning. Fix: on parse error, return an `anyhow::Error` with `.context()` pointing at the path and underlying parse error; never fall back to `DocumentMut::new()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 theme_cmd::update_toml_theme returns Err on parse failure instead of replacing doc
- [ ] #2 about_cmd::save_about_fields returns Err on parse failure
- [ ] #3 new_command_cmd::append_command_to_config returns Err on parse failure
- [ ] #4 Add test: malformed .ops.toml is not overwritten by each of the three subcommands
<!-- AC:END -->
