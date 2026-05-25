---
id: TASK-1292
title: >-
  ERR-5: save_about_fields replaces 'fields' array wholesale, discarding
  toml_edit decor
status: Done
assignee:
  - TASK-1303
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 17:58'
labels:
  - code-review-rust
  - error
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:91-98`

**What**: `save_about_fields` builds a fresh `toml_edit::Array` and calls `about.insert(\"fields\", toml_edit::value(arr))`, replacing any existing `fields` key wholesale. Inline comments and trailing decor on the prior array are lost.

**Why it matters**: `toml_edit` is chosen specifically to preserve user formatting and comments in `.ops.toml`. This codepath silently drops them on re-save, surprising users who annotated their config.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When 'fields' already exists, mutate the existing Array in place (clear + push) so decor is preserved
- [ ] #2 New-array path (key did not exist) is unchanged
- [ ] #3 Test pins that a trailing comment on 'fields = [...] # keep' survives a re-save
<!-- AC:END -->
