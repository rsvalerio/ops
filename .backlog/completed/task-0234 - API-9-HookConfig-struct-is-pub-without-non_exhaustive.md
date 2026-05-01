---
id: TASK-0234
title: 'API-9: HookConfig struct is pub without #[non_exhaustive]'
status: Done
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 07:44'
labels:
  - rust-code-review
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:14`

**What**: Adding a future field (e.g., group_header) would be a breaking change for downstream builders.

**Why it matters**: Extension authors outside the workspace pin construction of this struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add #[non_exhaustive] to HookConfig
- [x] #2 Provide a constructor to hide internals
<!-- AC:END -->
