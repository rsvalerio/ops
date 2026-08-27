---
id: TASK-1895
title: >-
  ERR-4: 'commands is not a table' names neither the .ops.toml path nor the
  offending key
status: Triage
assignee: []
created_date: '2026-08-27 15:35'
updated_date: '2026-08-27 15:36'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions/hook-common/src/config.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/config.rs:71-75`

**What**: The only hand-written error in `ensure_config_command` carries no location:

```rust
let commands = doc
    .entry("commands")
    .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
    .as_table_mut()
    .context("commands is not a table")?;
```

The operator sees `commands is not a table` and nothing else — not the file it came from, not that `commands` is a TOML top-level key rather than a CLI argument, and not what the value actually is (`commands = "verify"`, `commands = []`, an inline table). Every other error on this path names its path: `read_ops_toml` / `write_ops_toml` do (`ops_core::config`), and `install.rs` attaches `format!("failed to create temp hook in {}", parent.display())`-style context throughout. The path is already in scope one line up as `config_path`.

**Why it matters**: `ensure_config_command` runs as part of `ops <hook>-install`, which is typically the operator's *first* interaction with ops in a repo, and `config_dir` is a parameter — the `.ops.toml` it read is not necessarily the one in the cwd. A message that names neither the file nor the shape it expected leaves the operator grepping. ERR-13's reasoning applies directly: context that must be remembered at every callsite eventually is not, and this is the one callsite here that forgot. Suggested shape: `.with_context(|| format!("{}: top-level `commands` is not a table", config_path.display()))`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The 'commands is not a table' error names the .ops.toml path it was read from
- [ ] #2 A test writes an .ops.toml whose top-level commands key is a non-table and asserts the error text contains the file path
- [ ] #3 The message makes clear that the top-level TOML key "commands" is the offending key and states what shape was expected
<!-- AC:END -->
