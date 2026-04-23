---
id: TASK-0271
title: >-
  READ-8: hook_shared emits user-visible warning via writeln!(stderr) with ad
  hoc prefix
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 07:49'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:30`

**What**: Bypasses tracing/reporter used elsewhere; no level gating or structured fields.

**Why it matters**: Inconsistent output channel; counterpart to a sibling task at a different site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Route through tracing::warn or unified reporter
- [x] #2 Drop 'ops: warning:' prefix
<!-- AC:END -->
