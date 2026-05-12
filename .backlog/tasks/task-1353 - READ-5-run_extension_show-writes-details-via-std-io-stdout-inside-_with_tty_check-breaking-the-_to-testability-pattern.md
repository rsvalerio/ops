---
id: TASK-1353
title: >-
  READ-5: run_extension_show writes details via std::io::stdout() inside
  _with_tty_check, breaking the _to testability pattern
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:28'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:290`

**What**: `run_extension_show_with_tty_check` constructs a hardcoded `let mut w = std::io::stdout()` and calls `print_extension_details(&mut w, ...)` on the happy path. The list variant (`run_extension_list_to`) accepts `w: &mut dyn Write`, but the show path skips that seam.

**Why it matters**: Happy-path rendering of `ops extension show <name>` (the "EXTENSION:" header + field rows) cannot be asserted from tests against a buffer — only the no-extension / error branches are observable. Mirrors the gap that prompted TASK-1321 (`run_theme_select`) and TASK-1343 (`run_about_setup_with`); the show path is the third instance of the same family.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce run_extension_show_to (or thread w: &mut dyn Write into run_extension_show_with_tty_check) so the happy-path writes go through an injected writer
- [ ] #2 Add a unit test that captures the EXTENSION header + first field row from the _to entry point via a buffer (no stdout interception)
<!-- AC:END -->
