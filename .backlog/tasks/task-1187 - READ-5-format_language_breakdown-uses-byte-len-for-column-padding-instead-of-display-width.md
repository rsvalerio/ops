---
id: TASK-1187
title: >-
  READ-5: format_language_breakdown uses byte-len for column padding instead of
  display-width
status: To Do
assignee:
  - TASK-1271
created_date: '2026-05-08 08:11'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/format.rs:117`

**What**: `name_width = langs.iter().take(top_n).map(|l| short_language_name(&l.name).len()).max()` computes byte length and is fed into `{:<name_w$}` (which is char count). The codebase has standardised on display_width measurement (help.rs, tools_cmd.rs, theme_cmd.rs) precisely to avoid this drift.

**Why it matters**: All current language names are ASCII so the bug is dormant, but short_language_name's fallback returns the original name verbatim. A future language entry with non-ASCII name silently misaligns the codebase block.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_language_breakdown uses ops_core::output::display_width (and a manual space-pad loop) instead of .len() + {:<width$}.
- [ ] #2 Test: a synthetic LanguageStat with non-ASCII name aligns its column under an ASCII sibling at the same display column.
<!-- AC:END -->
