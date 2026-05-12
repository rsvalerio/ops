---
id: TASK-1355
title: >-
  READ-5: run_new_command_with_tty_check writes the 'Added command' confirmation
  to io::stdout() directly with no _to variant
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 21:28'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:47`

**What**: `run_new_command_with_tty_check` writes the success line `"Added command '{name}' to .ops.toml…"` via `writeln!(io::stdout(), ...)` rather than through an injected `&mut dyn Write`. No `_to` variant exists, so the happy path is only exercisable by spawning the binary.

**Why it matters**: Same _to-Writer seam gap as TASK-1321 (theme_select), TASK-1343 (about_setup), and the sibling extension_show finding — the production entry skips the writer-injection idiom adopted across tools_cmd / theme_cmd / extension_cmd / init_cmd, leaving the confirmation message unverifiable in unit tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add run_new_command_to_with_tty_check<W: Write>(... w: &mut W) (or thread w into the existing fn) and have the public shell pass &mut std::io::stdout()
- [ ] #2 Add a test that captures the 'Added command' confirmation from a buffer
<!-- AC:END -->
