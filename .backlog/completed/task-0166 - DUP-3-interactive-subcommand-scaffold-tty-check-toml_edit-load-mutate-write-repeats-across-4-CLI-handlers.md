---
id: TASK-0166
title: >-
  DUP-3: interactive-subcommand scaffold (tty check + toml_edit
  load/mutate/write) repeats across 4 CLI handlers
status: Done
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 14:14'
labels:
  - rust-code-review
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/cli/src/theme_cmd.rs:101-184` (run_theme_select + update_theme_in_config)
- `crates/cli/src/about_cmd.rs:15-91` (run_about_setup_with + save_about_fields)
- `crates/cli/src/new_command_cmd.rs:12-101` (run_new_command_with_tty_check + append_command_to_config)
- `crates/cli/src/hook_shared.rs:20-58` (run_hook_install) — partially already shared

**What**: Each interactive subcommand repeats the same scaffold: (1) `crate::tty::require_tty_with(...)`, (2) `inquire::<Select|Text|MultiSelect>...prompt()?`, (3) read `.ops.toml` (or empty if missing), (4) `content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|_| DocumentMut::new())`, (5) mutate a named table, (6) `fs::write(&config_path, doc.to_string())`. Steps 3-6 are identical except for the table name and the mutation closure. Steps 1-2 are also near-identical except for the prompt type and labels.

**Why it matters**: DUP-3 (3+ similar patterns). The duplicated write path also duplicates the ERR-5 bug flagged in TASK-0148/TASK-0153 (toml parse error swallowed) — fixing it once in a shared helper eliminates all four instances at once. Suggested refactor: `ops_core::config::edit_ops_toml<F>(mutate: F) -> anyhow::Result<()>` that handles read/parse-with-error/mutate/write atomically.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract shared edit_ops_toml helper that reads+parses+mutates+writes with proper error propagation
- [ ] #2 Refactor theme_cmd, about_cmd, new_command_cmd, hook-common to use the helper
<!-- AC:END -->
