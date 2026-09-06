---
id: TASK-1531
title: >-
  SEC-21: deps interpret_deny_result inconsistently trims stderr across
  truncate_for_log bails
status: Done
assignee:
  - TASK-1648
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 19:06'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:81-133`

**What**: At lines 113-115 the bail formats `truncate_for_log(stderr)` without `.trim()`, while sibling bails at 121-122 and 130-132 use `.trim()` (or don't). Untrimmed stderr can carry leading/trailing newlines and control bytes that — though Debug-escaped — bloat the error message.

**Why it matters**: Mostly cosmetic, but consistency matters here: the file mixes trimmed and untrimmed truncation, and the next contributor will pick the wrong one. Distinct from TASK-1250 (control-byte scrubbing) which is already Done.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Normalise to one rule (trim before truncate) across all four bail sites
- [ ] #2 All bails Debug-format ({:?}) the result — already true
- [ ] #3 No regression in interpret_deny_result_* tests
<!-- AC:END -->
