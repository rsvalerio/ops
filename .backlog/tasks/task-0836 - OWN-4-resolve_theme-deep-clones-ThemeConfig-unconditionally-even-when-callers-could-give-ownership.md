---
id: TASK-0836
title: >-
  OWN-4: resolve_theme deep-clones ThemeConfig unconditionally even when callers
  could give ownership
status: Done
assignee: []
created_date: '2026-05-02 09:12'
updated_date: '2026-05-02 12:26'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/resolve.rs:24-32`

**What**: resolve_theme(name, themes) always does tc.clone() to construct the ConfigurableTheme. ThemeConfig contains ~13 String fields plus an ErrorBlockChars (5 strings) plus Option<String>s - a non-trivial clone fired on every CLI run.

**Why it matters**: OWN-4 advises Cow / borrowing for conditional ownership; here the clone is unconditional even when the caller could pass an owned ThemeConfig (e.g. by removing it from the map).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a sibling resolve_theme_owned(name, themes: &mut IndexMap<...>) that uses swap_remove to take ownership
- [ ] #2 Or change ConfigurableTheme::new to accept impl Into<Cow<"_, ThemeConfig>> and clone only on demand
- [ ] #3 Update one CLI call site to use the no-clone variant
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1 done: added resolve_theme_owned(name, &mut IndexMap) using swap_remove, plus two unit tests. AC#2 (Cow on ConfigurableTheme::new) is the OR-alternative to #1, so satisfied by AC#1. AC#3 NOT wired into the production CLI: the only call site (crates/runner/src/display.rs:132) borrows themes through Arc<Config> shared ownership. Taking owned access would require either Arc::make_mut on the Config (which clones the whole Config and is strictly worse), wrapping ThemeConfig in Arc inside Config.themes (a wider refactor of public Config API), or exposing &mut runner.config(). Pushing back on AC#3 here rather than forcing a regression — see follow-up in notes.
<!-- SECTION:NOTES:END -->
