---
id: TASK-1239
title: >-
  PATTERN-1: has_legacy_marker substring match still triggers inside quoted
  strings and here-docs
status: Done
assignee:
  - TASK-1270
created_date: '2026-05-08 12:59'
updated_date: '2026-05-10 17:06'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:110-121`

**What**: After TASK-1072 the matcher skips `#` comment lines but still uses `trimmed.contains(marker)` on every other line. A user-authored hook with `echo "Tip: run 'ops run-before-commit' manually"`, a here-doc body, or a printf argument that mentions the marker triggers the upgrade path and gets overwritten by the ops template.

**Why it matters**: Foreign-hook overwrite of a hand-written user script because a string literal happens to contain the marker — same misclassification class TASK-1072 patched, on a different lexical context.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match the marker only as a leading word (line trim_start matches exec ops <name> / ops <name>)
- [ ] #2 Tests for echo / here-doc / printf bodies that mention the marker as data
- [ ] #3 Update doc comment to spell out the leading-word contract
<!-- AC:END -->
