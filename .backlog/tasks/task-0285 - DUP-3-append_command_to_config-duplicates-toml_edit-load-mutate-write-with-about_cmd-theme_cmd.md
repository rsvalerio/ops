---
id: TASK-0285
title: >-
  DUP-3: append_command_to_config duplicates toml_edit load-mutate-write with
  about_cmd/theme_cmd
status: To Do
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:61`

**What**: Three CLI handlers share parse-ensure_table-insert-write plumbing.

**Why it matters**: Not covered by the tty/scaffold dup task nor the unwrap_or_else parse-swallow task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract load_edit_write(path, mutate_fn) helper
- [ ] #2 Use from about_cmd/new_command_cmd/theme_cmd
<!-- AC:END -->
