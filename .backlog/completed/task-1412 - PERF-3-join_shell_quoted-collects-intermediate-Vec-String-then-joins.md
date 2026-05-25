---
id: TASK-1412
title: 'PERF-3: join_shell_quoted collects intermediate Vec<String> then joins'
status: Done
assignee:
  - TASK-1457
created_date: '2026-05-13 18:17'
updated_date: '2026-05-14 08:00'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:200`

**What**: `join_shell_quoted` maps each part through `shell_quote(...).into_owned()` into a `Vec<String>` then calls `.join(" ")`, allocating one `String` per arg plus the intermediate `Vec<String>` plus the final join. Called from `display_cmd` and `expanded_args_display` on every dry-run / step-line render of an Exec command.

**Why it matters**: Hot on dry-run preview and step-line render paths for every Exec command with args. `shell_quote` returns `Cow<str>` letting the safe-arg case stay borrowed, but `into_owned()` discards that win. Write directly into a single pre-sized output `String` and push `shell_quote(...)`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rewrite join_shell_quoted to push directly into a single pre-sized String without an intermediate Vec<String>
- [ ] #2 safe-arg fast path borrows from shell_quote's Cow::Borrowed without allocating
- [ ] #3 preserve existing rendering exactly (covered by existing display_cmd tests)
<!-- AC:END -->
