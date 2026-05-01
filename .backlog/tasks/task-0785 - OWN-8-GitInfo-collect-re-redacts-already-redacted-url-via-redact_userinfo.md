---
id: TASK-0785
title: 'OWN-8: GitInfo::collect re-redacts already-redacted url via redact_userinfo'
status: To Do
assignee:
  - TASK-0827
created_date: '2026-05-01 05:58'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/provider.rs:64`

**What**: read_origin_url_from (config.rs:63) already runs redact_userinfo(value) before returning. The fallback branch in provider.rs:64 calls config::redact_userinfo(&raw) again on the same string. The defense-in-depth comment is fine, but the duplicated allocation is unnecessary and two redaction sites maintaining the same invariant make a future change at one site silently miss the other.

**Why it matters**: Minor wasted work; more importantly, two redaction sites = drift risk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove the second redact_userinfo call OR move all redaction to a single layer with a comment explaining why
- [ ] #2 Add a regression test pinning that the value returned by read_origin_url_from is already redacted (so the provider trusts it)
<!-- AC:END -->
