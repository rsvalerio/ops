---
id: TASK-0602
title: >-
  ERR-2: severity_icon and colorize_severity collapse all unknown severities to
  dim/info
status: Triage
assignee: []
created_date: '2026-04-29 05:19'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:8`

**What**: Both helpers default to info icon (\u{2139}) and dim color for any severity outside error/warning. Combined with parse_deny_output defaulting missing severities to "error", an entry with an explicit non-standard severity (e.g. cargo-deny adds critical) prints with the lowest-emphasis style. No warn log either — schema drift invisible.

**Why it matters**: Operator`s eye is trained on icon set; a critical advisory rendering as info is exactly the misclassification the formatter is supposed to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Unknown severities log once per render and use a clearly distinct fallback (e.g. red question-mark icon)
<!-- AC:END -->
