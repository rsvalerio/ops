---
id: TASK-1346
title: >-
  READ-2: ThemeOption has two divergent rendering surfaces — Display impl (theme
  select prompt) and run_theme_list_to (theme list)
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 16:42'
updated_date: '2026-05-12 23:23'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:100-114, 189-194`

**What**: The same `ThemeOption` struct is rendered two ways:
- `theme list` (lines 100-114): `{padded_name}   {description}{marker}` — three-space separator, marker last
- `theme select` prompt (`Display` impl, lines 189-194): `{name}{marker} - {description}` — `" - "` separator, marker between name and description, no padding

**Why it matters**: Two operator-facing surfaces show the same data in inconsistent layouts — users running `ops theme list` and then `ops theme select` see two formats for what they perceive as the same theme line. Pick one canonical form (or share a formatter helper) so the rendering is predictable across surfaces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ThemeOption renders the same shape (name, marker, description) in both list and select surfaces, via a shared formatter or by reusing Display in run_theme_list_to
- [ ] #2 Snapshot/integration tests cover both surfaces
<!-- AC:END -->
