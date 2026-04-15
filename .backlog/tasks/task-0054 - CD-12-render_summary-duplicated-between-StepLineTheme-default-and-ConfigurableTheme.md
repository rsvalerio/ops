---
id: TASK-0054
title: >-
  CD-12: render_summary duplicated between StepLineTheme default and
  ConfigurableTheme
status: Done
assignee: []
created_date: '2026-04-14 20:32'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-1
  - DUP-5
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/theme/src/lib.rs` — trait default `render_summary` and `ConfigurableTheme::render_summary`
**Anchor**: `fn render_summary`
**Impact**: The trait default implementation ignores `self.summary_prefix()` which it already provides as a method, so ConfigurableTheme overrides render_summary solely to inject the prefix. If the trait default called `self.summary_prefix()`, the override would be unnecessary and the two near-identical formatting blocks would collapse to one.

DUP-1: near-identical code blocks in the same file. DUP-5: the trait default should call its own summary_prefix() method.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_summary is defined once in the trait default using self.summary_prefix(); ConfigurableTheme no longer overrides it
<!-- AC:END -->
