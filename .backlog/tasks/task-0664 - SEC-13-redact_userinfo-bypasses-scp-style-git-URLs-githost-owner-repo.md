---
id: TASK-0664
title: 'SEC-13: redact_userinfo bypasses scp-style git URLs (git@host:owner/repo)'
status: To Do
assignee:
  - TASK-0743
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:54-67` and `extensions/git/src/provider.rs:64`

**What**: `redact_userinfo` only handles URLs with a `://` scheme; scp-style remotes (`git@host:owner/repo`) bypass redaction entirely. The unparseable-fallback branch at provider.rs:64 (`Some(config::redact_userinfo(&raw))`) emits scp-style raw URLs verbatim into JSON output and logs.

**Why it matters**: `parse_remote_url` rejects malformed scp URLs (e.g. ones containing extra `:` or unsafe owner/repo segments), at which point the *raw* string is propagated. A `[remote "origin"]` written by an external tool with `user:tok@host:path/garbage` would be unparseable, hit the fallback, skip the `://` short-circuit, and surface the userinfo intact in `git_info.remote_url`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extend redact_userinfo to handle the scp form: detect a leading …@ segment before the first : in non-:// inputs and strip it
- [ ] #2 Regression test: feed an unparseable scp value with user:tok@host:weird and assert the fallback URL has no user:tok / @
<!-- AC:END -->
