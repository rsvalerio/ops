---
id: TASK-0801
title: >-
  DUP-1: format_advisories and format_deny_section share the
  title+count+pkg-width+advice block rendering shape
status: Triage
assignee: []
created_date: '2026-05-01 06:01'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:75-95, 155-188, 227-257`

**What**: After TASK-0610 unified the empty-section path, the non-empty path is split between format_advisories and format_deny_section. Both: print title with count, compute pkg_width = max(...), iterate entries with colorize_severity(icon, sev), then emit one or more dim advice lines. The structural difference is only the id column.

**Why it matters**: A future change to advice formatting, severity rendering, or column layout has to be made twice. The reason format_deny_section is generic over T + extract is exactly the abstraction format_advisories needs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either extend format_deny_section extractor to optionally return an id column, or fold both into a format_severity_section helper parameterised over (title, entries, columns, advice)
- [ ] #2 format_advisories and format_deny_section collapse to call sites of the new helper
- [ ] #3 Snapshot/golden test confirms output unchanged for the existing four sections
<!-- AC:END -->
