---
id: TASK-0972
title: 'DUP-3: severity_icon and colorize_severity duplicate the severity-class table'
status: Done
assignee: []
created_date: '2026-05-04 21:48'
updated_date: '2026-05-04 23:02'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:30-56`

**What**: `severity_icon` and `colorize_severity` each maintain an independent match arm over `("error", "warning", "note"|"help"|"info")`. Adding a new severity (e.g. `critical`) requires editing both and risks one falling out of sync — exactly the drift the unknown-severity warn (TASK-0602) detects after the fact.

**Why it matters**: Two tables, one source of truth missing. Refactor risk; harmless today but invites subtle inversions where the icon and color disagree on classification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single SeverityClass enum with from_str, icon, and style accessors
- [ ] #2 Both functions consume the enum; unknown severities classify into a single fallback variant
- [ ] #3 Test asserts every known severity round-trips through both helpers identically
<!-- AC:END -->
