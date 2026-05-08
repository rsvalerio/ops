---
id: TASK-1237
title: >-
  PATTERN-1: parse_remote_url silently rewrites http/git/ssh remotes to https in
  the synthesized url field
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 12:59'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:35-58`

**What**: `parse_remote_url` accepts http, ssh, git, and https in `ALLOWED_SCHEMES` but unconditionally synthesises `https://{host}/{owner}/{repo}` as `RemoteInfo.url`. A repo whose origin is `http://internal/...` or `git://anon/...` ends up advertising an https URL, and downstream consumers (about cards, JSON output, audit logs) get a misleading scheme.

**Why it matters**: Mis-attribution of remote scheme — clickable links land on a different protocol than the actual remote, and audit/policy code distinguishing http vs https treats the result as TLS-fronted when it was not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Preserve original scheme when synthesising the url field
- [ ] #2 Update tests to pin scheme round-trip for http/ssh/git inputs
- [ ] #3 Document the scheme-preservation contract in RemoteInfo
<!-- AC:END -->
