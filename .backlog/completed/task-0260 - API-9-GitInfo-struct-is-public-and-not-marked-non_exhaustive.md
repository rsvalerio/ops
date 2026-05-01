---
id: TASK-0260
title: 'API-9: GitInfo struct is public and not marked #[non_exhaustive]'
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 07:47'
labels:
  - rust-code-review
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/provider.rs:13`

**What**: External consumers may pattern-match exhaustively; adding a field becomes a breaking change.

**Why it matters**: Locks future extension of the data provider shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Annotate GitInfo with #[non_exhaustive]
- [x] #2 Also apply to RemoteInfo
<!-- AC:END -->
