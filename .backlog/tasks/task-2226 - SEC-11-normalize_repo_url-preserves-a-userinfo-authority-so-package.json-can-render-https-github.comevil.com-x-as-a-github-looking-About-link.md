---
id: TASK-2226
title: 'SEC-11: normalize_repo_url preserves a userinfo authority, so package.json can render https://github.com@evil.com/x as a github-looking About link'
status: Done
assignee: []
created_date: '2026-09-08 07:22'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2236'
modified_files:
  - extensions-node/about/src/repo_url.rs
priority: medium
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:71`, `extensions-node/about/src/repo_url.rs:241`

**What**: `normalize_repo_url`'s only post-rewrite gate is
`has_allowed_url_scheme`, which (see `extensions/about/src/text_util.rs:94`)
compares the leading bytes against `http://` / `https://` and inspects nothing
after the scheme. The authority itself is never validated:

- the clean-URL fall-through (`repo_url.rs:135`) returns the trimmed input
  verbatim, so `https://github.com@evil.com/owner/repo` passes;
- `scrub_authority_and_path` (`repo_url.rs:241`) deliberately keeps the
  leading segment "verbatim", so the `git://` and `git+<scheme>://` branches
  rewrite `git://github.com@evil.com/o/r` into
  `https://github.com@evil.com/o/r`.

Everything before the `@` is RFC 3986 userinfo; the effective host is
`evil.com`. Rendered into the About card the link reads as a github.com URL.

**Why it matters**: this crate already treats `package.json::repository` as
adversarial input and drops the field rather than emit a misleading link, for
control bytes (SEC-2 / TASK-1165), path traversal (SEC-14 / TASK-1111), a
hostless authority (TASK-1256), and non-http schemes (SEC-11 / TASK-1722).
Userinfo spoofing produces exactly the outcome those fixes exist to prevent —
a clickable, plausible-looking URL pointing at an attacker-chosen host — and
is the one authority-shaped defect the current checks let through.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 a repository URL whose authority contains userinfo (user@host or user:pass@host) is dropped, matching the existing drop-the-field policy
- [ ] #2 the rule applies to the clean-URL fall-through and to every rewrite branch that routes through scrub_authority_and_path
- [ ] #3 legitimate authorities including a numeric port (host:22) continue to round-trip unchanged
- [ ] #4 tests pin https://github.com@evil.com/o/r and git://github.com@evil.com/o/r as dropped
<!-- AC:END -->
