---
id: TASK-1343
title: >-
  READ-5: run_about_setup_with writes happy-path output via std::io::stdout(),
  breaking the _to testability pattern
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 16:41'
updated_date: '2026-05-12 23:23'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:71-75`

**What**: `run_about_setup_with` selects fields interactively and then writes the confirmation message via `writeln!(std::io::stdout(), ...)` instead of a `&mut dyn Write` parameter. Other handlers in the crate (`tools_cmd`, `theme_cmd::run_theme_list_to`, `init_cmd`) follow a `_to(...)` shape that accepts an injected writer for test capture.

**Why it matters**: The happy-path output cannot be asserted by buffer-feed tests — only by stdout-capture (`assert_cmd`), which is more brittle and less precise. Mirrors the previously-filed TASK-1321 finding for `run_theme_select` but at a distinct call site; keeps the in-crate testing pattern from being undermined twice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_about_setup_with takes a &mut dyn Write for confirmation output (or grows a _to variant) and tests cover the message
- [ ] #2 Caller in dispatch threads io::stdout() through; cargo test passes
<!-- AC:END -->
