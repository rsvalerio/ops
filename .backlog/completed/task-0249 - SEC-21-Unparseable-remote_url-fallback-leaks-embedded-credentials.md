---
id: TASK-0249
title: 'SEC-21: Unparseable remote_url fallback leaks embedded credentials'
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 07:45'
labels:
  - rust-code-review
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/provider.rs:41`

**What**: When parse_remote_url fails, GitInfo.remote_url is set to the raw origin url which may contain user:token@ userinfo.

**Why it matters**: Credentials end up in data-provider JSON consumed by logs/downstream tools.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Strip userinfo component from raw string before falling back
- [x] #2 Add test that raw https://user:tok@host/weird returns credential-free remote_url
<!-- AC:END -->
