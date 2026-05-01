---
id: TASK-0809
title: >-
  ERR-7: tracing::warn! logs go.work use directive via Display, allowing log
  injection
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:02'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:38`

**What**: tracing::warn!(directive = %dir, ...) formats dir (parsed from a manifest controlled by repo content) with %% (Display) — newlines or ANSI sequences in a use-dir line propagate verbatim to logs.

**Why it matters**: Same class as TASK-0665 (workspace.rs path log fields); newly-introduced occurrence in the Go modules provider missed in that task scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Switch to ?dir (Debug) or tracing::field::display with sanitization, matching the project convention from TASK-0665
- [ ] #2 Test pins that a directive containing newline does not produce a multi-line log record
<!-- AC:END -->
