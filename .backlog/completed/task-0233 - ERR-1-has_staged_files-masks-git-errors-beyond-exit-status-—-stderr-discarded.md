---
id: TASK-0233
title: 'ERR-1: has_staged_files masks git errors beyond exit status — stderr discarded'
status: Done
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 07:36'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:62`

**What**: `.output()` captures stderr but `Ok(!output.stdout.is_empty())` never inspects it; missing git binary or broken repo returns Ok(false).

**Why it matters**: Silent git failures are indistinguishable from "no staged files".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Include stderr snippet in returned error on non-success
- [x] #2 Add regression test for git not found
<!-- AC:END -->
