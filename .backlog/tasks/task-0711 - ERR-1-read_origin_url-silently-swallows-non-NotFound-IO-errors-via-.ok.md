---
id: TASK-0711
title: 'ERR-1: read_origin_url silently swallows non-NotFound IO errors via .ok()?'
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:29'
updated_date: '2026-04-30 18:06'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:9`

**What**: `read_origin_url` calls `std::fs::read_to_string(git_dir.join("config")).ok()?`. The `.ok()?` chain coerces every IO error (PermissionDenied, IsADirectory, partial read) to `None`, indistinguishable from "remote.origin.url is not set". A user with an unreadable .git/config (wrong owner, ACL drift) sees identical behaviour to a fresh repo with no remotes.

**Why it matters**: This is the same swallow-non-NotFound pattern caught in sibling code by TASK-0517 (resolve_member_globs) and TASK-0548 (try_read_manifest), which now log at `tracing::warn`. Keeping `read_origin_url` on `.ok()?` lets a real broken config silently degrade `about`/`provider` output to "no remote", masking diagnoseable issues.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match on the io::Error: NotFound returns None silently; other errors log at warn with the .git/config path before returning None
- [ ] #2 Mirror the policy already established by try_read_manifest and resolve_member_globs (TASK-0548 / TASK-0517)
- [ ] #3 Add a unit test (cfg(unix)) that pins: an unreadable .git/config returns None and emits a tracing event
<!-- AC:END -->
