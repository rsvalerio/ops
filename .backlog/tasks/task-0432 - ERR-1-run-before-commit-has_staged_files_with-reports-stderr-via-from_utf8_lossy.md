---
id: TASK-0432
title: >-
  ERR-1: run-before-commit has_staged_files_with reports stderr via
  from_utf8_lossy
status: Done
assignee:
  - TASK-0535
created_date: '2026-04-28 04:42'
updated_date: '2026-04-28 13:45'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:60`

**What**: String::from_utf8_lossy(&output.stderr) silently substitutes U+FFFD for any invalid UTF-8 in git stderr before bubbling it into the user-facing anyhow! chain.

**Why it matters**: Diagnostic-only — git stderr is overwhelmingly UTF-8 in practice — but the lossy conversion drops information from the error message in exactly the case the user is most likely trying to debug (a misbehaving git binary).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either bail with the raw bytes (format!("{:?}", output.stderr)) or document that lossy is intentional with a comment plus a unit test pinning the behavior
<!-- AC:END -->
