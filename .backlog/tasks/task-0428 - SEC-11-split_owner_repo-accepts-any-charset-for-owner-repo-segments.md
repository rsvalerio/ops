---
id: TASK-0428
title: 'SEC-11: split_owner_repo accepts any charset for owner/repo segments'
status: To Do
assignee:
  - TASK-0535
created_date: '2026-04-28 04:42'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:103-114`

**What**: Hosts pass a strict allowlist (is_valid_host: ASCII alphanumeric plus `.` `-`), but owner/repo segments are accepted as whatever falls between path slashes. Inputs like `https://github.com/own\'er/<script>repo` produce a RemoteInfo { owner: "own\'er", repo: "<script>repo", url: "https://github.com/own\'er/<script>repo" } that flows into GitInfo JSON and downstream about rendering.

**Why it matters**: Defense-in-depth asymmetry — the host slot is locked down precisely because it lands in a https://{host}/... URL, but the owner/repo slot lands there too with no charset constraint, undermining the protection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_remote_url rejects owner/repo segments containing characters outside a documented allowlist (alphanumerics, ., -, _, plus ~ for sourcehut-style users)
- [ ] #2 Existing valid inputs (sourcehut ~user, GitLab nested groups, scp-style) continue to parse
- [ ] #3 Regression test demonstrates that an injected quote/angle bracket no longer produces a RemoteInfo
<!-- AC:END -->
