---
id: TASK-0894
title: >-
  ARCH-2: extensions/git provider relies on undocumented redaction invariant of
  read_origin_url_from
status: Done
assignee: []
created_date: '2026-05-02 09:47'
updated_date: '2026-05-02 14:45'
labels:
  - code-review-rust
  - architecture
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/git/src/provider.rs:64

**What**: The fallback branch was changed from Some(config::redact_userinfo(&raw)) to Some(raw) (per TASK-0785) on the assumption that read_origin_url_from already redacts. That invariant is enforced only by a sibling test, not by the type system or the function signature/doc.

**Why it matters**: If a future refactor changes read_origin_url_from to return a raw url (e.g. a new code path bypassing the redactor), credentials leak into GitInfo.remote_url and downstream into about-cards / JSON output. SEC regression risk is silent and high.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a doc-comment contract on read_origin_url_from declaring the userinfo-redacted return invariant
- [x] #2 Wrap the return type in a newtype (e.g. RedactedUrl) so the type system enforces the invariant, OR re-apply redact_userinfo defensively here (it is idempotent and cheap)
- [x] #3 Add a doc-test demonstrating the invariant on the public surface
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Introduced ops_git::config::RedactedUrl newtype. read_origin_url and read_origin_url_from now return Option<RedactedUrl>; the only constructor (RedactedUrl::redact / From<&str>) routes through redact_userinfo, so a future refactor that returns a raw URL becomes a compile error. Updated provider.rs to call .as_str() / .into_string() at the boundary. Added a doc-test on RedactedUrl::redact demonstrating the userinfo-redaction invariant + idempotence.
<!-- SECTION:NOTES:END -->
