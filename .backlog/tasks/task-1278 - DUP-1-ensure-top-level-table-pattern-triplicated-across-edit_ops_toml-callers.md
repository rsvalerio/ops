---
id: TASK-1278
title: >-
  DUP-1: 'ensure top-level table' pattern triplicated across edit_ops_toml
  callers
status: Done
assignee:
  - TASK-1303
created_date: '2026-05-11 15:25'
updated_date: '2026-05-11 17:58'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:87`, `crates/cli/src/new_command_cmd.rs:104`, `crates/cli/src/theme_cmd.rs:197`

**What**: Three call sites repeat the same 4-line idiom: check `doc.contains_key(K)`, insert an empty `toml_edit::Table` if missing, then `doc[K].as_table_mut().context(...)`. Distinct from TASK-1273 (which targets gather_available_commands).

**Why it matters**: Each repetition is a fresh chance to forget the `as_table_mut().context(...)` arm — and one of the three (`theme_cmd::set_theme`) silently writes through `doc["output"]["theme"]` indexer without validating that `[output]` is actually a table, which would panic in toml_edit if it is the wrong kind. A single helper enforces the invariant once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Helper like fn ensure_table<'a>(doc: &'a mut DocumentMut, key: &str) -> anyhow::Result<&'a mut toml_edit::Table> lives near edit_ops_toml
- [ ] #2 All three sites call the helper and the inline contains_key + insert + as_table_mut().context(...) block disappears
- [ ] #3 set_theme uses the helper for [output] and no longer relies on the panicking IndexMut path
- [ ] #4 Existing tests for each site pass without modification
<!-- AC:END -->
