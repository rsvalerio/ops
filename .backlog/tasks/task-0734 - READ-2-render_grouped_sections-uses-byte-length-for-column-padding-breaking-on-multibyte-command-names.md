---
id: TASK-0734
title: >-
  READ-2: render_grouped_sections uses byte length for column padding, breaking
  on multibyte command names
status: To Do
assignee:
  - TASK-0742
created_date: '2026-04-30 05:50'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:144-165` (`render_grouped_sections`)

**What**: Help rendering computes `max_name_width = entries.iter().map(|e| e.name.len()).max()` and uses it as the column width via `{:<width$}`. `String::len` returns *bytes*, not display columns. For any command name containing multibyte characters (CJK, emoji, accented identifiers — e.g. an extension or user-defined command named `ビルド` or `🚀deploy`), the padding under-counts and the help table mis-aligns.

The project already has the right helper: `ops_core::output::display_width` (`crates/core/src/output.rs:8`) which delegates to `unicode_width::UnicodeWidthStr` and is used elsewhere for this exact purpose (`crates/theme/src/step_line_theme.rs`, `crates/runner/src/display.rs`).

On top of the width bug, `format!("  {:<width$}  {}\n", entry.name, ...)` with a `width` derived from byte count and a `{:<width$}` formatter sized in `char` count means even ASCII names with combining marks would mis-align (today no built-in commands hit this, but extension-provided commands are unrestricted).

**Why it matters**: Low-impact while command names are ASCII, but commands are user/extension-supplied (`config.commands`, `register_extension_commands`) so the safe-against-future-input fix is mechanical.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_grouped_sections computes max width via display_width(&entry.name) (or unicode_width directly) instead of String::len
- [ ] #2 Padding logic accounts for the possibility that display_width != char count (use a manual pad with spaces rather than {:<width$})
- [ ] #3 Add a regression test for a command name containing wide characters (e.g. include the test only when an extension/test config defines such a name)
<!-- AC:END -->
