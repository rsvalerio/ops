---
id: TASK-0944
title: >-
  ERR-7: text::for_each_trimmed_line warn uses Display for path/error (log
  injection via attacker-controlled manifest path)
status: Done
assignee: []
created_date: '2026-05-02 16:03'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:60-65`

**What**: The warn event for an unreadable manifest logs `path = %path.display(), error = %e` with Display. Path is a stack-manifest path (`go.mod`, `gradle.properties`, `requirements.txt`, etc.) under user-controlled CWD.

**Why it matters**: Sister to TASK-0930/0937. Distinct from TASK-0932 which addresses the byte-cap gap on this same function — this finding is purely the formatter. Embedded newlines/ANSI in a manifest path can forge or corrupt operator-facing tracing output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch path and error fields in the warn event to Debug formatter
- [x] #2 Regression test asserts an embedded \n or \u{1b} in the manifest path is escaped in tracing output
<!-- AC:END -->
