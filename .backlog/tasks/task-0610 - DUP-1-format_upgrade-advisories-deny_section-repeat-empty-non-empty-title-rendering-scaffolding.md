---
id: TASK-0610
title: >-
  DUP-1: format_upgrade/advisories/deny_section repeat empty/non-empty title
  rendering scaffolding
status: Triage
assignee: []
created_date: '2026-04-29 05:20'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:72`

**What**: Five formatters share the pattern: empty branch prints "title green ✓ None"; non-empty branch prints "title (count):", computes pkg_width, iterates with severity icon + colorized name + dim message, then prints "💡 advice". Five candidates (format_upgrade_section, format_advisories, format_deny_section x2, format_bans_summary) duplicate the empty-state render exactly.

**Why it matters**: Formatting drift is a persistent source of bugs in this crate (TASK-0189 family). Extracting a format_section_header helper localises future style tweaks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Empty-state line produced by single helper invoked by all section formatters
<!-- AC:END -->
