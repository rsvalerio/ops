---
id: TASK-1072
title: >-
  PATTERN-1: hook-common has_legacy_marker substring match triggers on
  commented-out marker text
status: Done
assignee: []
created_date: '2026-05-07 21:19'
updated_date: '2026-05-08 06:28'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:96-101`

**What**: `has_legacy_marker` does `content.contains(marker)`, which matches inside comments and quoted strings. A user-authored hook containing `# legacy: ops run-before-commit (do not use)` would trip the legacy-marker check, and the upgrade flow would overwrite the user's script with the ops template.

**Why it matters**: User script overwrite on a false-positive substring. The `upgrade_legacy_hook` double-check mitigates a TOCTOU race but not this false positive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Match on uncommented script lines (skip lines beginning with #) before substring containment
- [x] #2 Anchor the marker to a startswith / whole-line check rather than naked contains()
- [x] #3 Add a test where a comment containing 'ops run-before-commit' does NOT trigger upgrade
<!-- AC:END -->
