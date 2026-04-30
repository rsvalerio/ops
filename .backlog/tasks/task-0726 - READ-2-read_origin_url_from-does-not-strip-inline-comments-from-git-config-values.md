---
id: TASK-0726
title: >-
  READ-2: read_origin_url_from does not strip inline ; / # comments from
  git-config values
status: To Do
assignee:
  - TASK-0743
created_date: '2026-04-30 05:47'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:28-47` (read_origin_url_from), `:69-76` (strip_url_key)

**What**: The line scanner skips lines whose *first* non-whitespace char is `#` or `;`, but git-config also supports inline trailing comments — `url = https://example.com/x.git ; old value` is a legal git-config line and `git config --get remote.origin.url` returns just `https://example.com/x.git`. `strip_url_key` returns the entire RHS verbatim, so our parser emits `https://example.com/x.git ; old value` as the origin URL. That string then flows into `redact_userinfo` (no-op — no `@`), `parse_remote_url` (rejected as invalid host), and finally the unparseable-fallback branch in `provider.rs:64`, which publishes the comment-bearing string as `git_info.remote_url`.

**Why it matters**: Sibling parsers in the codebase strip inline comments (TASK-0496 go.mod, TASK-0497 go.work, TASK-0509 gradle, TASK-0624 settings.gradle) and this one was missed. The visible symptom is a `remote_url` field with stray `; old…` text appearing in provenance metadata or report output — confusing rather than catastrophic, but it diverges from `git config --get`, which is the documented contract of the parser. Filed as low because git-config inline comments are rare in real-world remotes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 strip_url_key (or read_origin_url_from) trims trailing #/; comments from the value, matching git-config(1) semantics
- [ ] #2 doc comment on read_origin_url_from updates the limitations list to remove the inline-comment caveat
- [ ] #3 test covers a config line of the form 'url = https://x.example/r.git ; comment' and asserts the comment is removed
<!-- AC:END -->
