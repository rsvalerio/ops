---
id: TASK-1526
title: >-
  READ-5: deps severity_icon/colorize_severity are wrappers that re-classify per
  call
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-19 07:33'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:71-86`

**What**: Both free functions exist solely to call `SeverityClass::classify` then dispatch — but `format_severity_section` calls both per row, classifying the same severity string twice. The wrappers obscure that the section formatter could classify once and call `.icon()` / `.style()` directly.

**Why it matters**: TASK-1495 (PERF-3 double-classify) is already filed; the underlying readability/structural cause — needless wrapper functions over the enum — is what makes the duplicate call easy to miss. Removing the wrappers fixes the perf issue at the call-site by construction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inline SeverityClass::classify at the call sites that need both icon and style; drop the two wrappers
- [ ] #2 Single classify per row in format_severity_section
- [ ] #3 helper_tests updated to call the enum methods directly
<!-- AC:END -->
