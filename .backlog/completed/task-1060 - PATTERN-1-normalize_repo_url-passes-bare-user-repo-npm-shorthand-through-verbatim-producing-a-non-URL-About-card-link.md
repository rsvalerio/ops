---
id: TASK-1060
title: >-
  PATTERN-1: normalize_repo_url passes bare user/repo npm shorthand through
  verbatim, producing a non-URL About card link
status: Done
assignee: []
created_date: '2026-05-07 21:04'
updated_date: '2026-05-08 06:16'
labels:
  - code-review
  - triage
  - extensions-node
  - repo_url
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-node/about/src/repo_url.rs:16-43 normalize_repo_url handles git+ssh, ssh://, git+https, git://, npm host shorthands (github:/gitlab:/bitbucket:), and trailing .git but treats unrecognised input as plain text via the final `s.trim_end_matches(".git").to_string()` arm. npm package.json semantics accept a bare `user/repo` shorthand (defaults to GitHub) — e.g. `"repository": "expressjs/express"` — which currently surfaces in the About card as the literal string `expressjs/express` rather than `https://github.com/expressjs/express`. The card renders an unclickable identifier, defeating the repository field. Add a dedicated branch that recognises a bare two-segment slug shape (no scheme, no colon, exactly one /) and rewrites it to the github.com URL, mirroring the existing `github:owner/repo` shorthand path. Add a regression test covering `expressjs/express` and an org-scoped `@scope/name` (which is a package name, not a repo shorthand, and should NOT be rewritten — verify it falls through unchanged).
<!-- SECTION:DESCRIPTION:END -->
