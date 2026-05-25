---
id: TASK-1151
title: >-
  SEC-13: GitInfo fallback branch ships unparseable post-redaction garbage as
  remote_url
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 07:43'
updated_date: '2026-05-08 14:04'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/provider.rs:66-83`

**What**: When parse_remote_url(raw.as_str()) returns None, the provider falls back to Some(raw.into_string()) as remote_url. RedactedUrl::redact strips userinfo and (TASK-1102) drops control bytes, but does not require the result to be a meaningful URL — pathological scp-style values like `weird@value/with@embedded` get trimmed to `embedded` and shipped as remote_url.

**Why it matters**: SEC-2 / TASK-1102 already adopts a fail-closed posture for control-byte values. Same posture is missing for \"redacted form parses as garbage\".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When parse_remote_url returns None in GitInfo::collect, drop remote_url entirely (set to None)
- [ ] #2 Or tighten RedactedUrl::redact to require a recognised scheme prefix (https://, ssh://, git@) and return None otherwise
<!-- AC:END -->
