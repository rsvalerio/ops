---
id: TASK-0261
title: 'SEC-11: parse_remote_url accepts any scheme/host charset without allowlist'
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 07:47'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:42`

**What**: No scheme allowlist (could be file://, javascript: etc.) and host chars not validated; host is interpolated into normalized URL.

**Why it matters**: git config is attacker-influenceable; downstream consumers may treat host as safe for links/commands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Enforce scheme ∈ {https, http, ssh, git}
- [x] #2 Validate host per RFC 3986 host charset; add rejection tests
<!-- AC:END -->
