---
id: TASK-1205
title: >-
  SEC-14: Bare owner/repo shorthand branch in normalize_repo_url skips
  path-traversal scrub
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 08:16'
updated_date: '2026-05-08 14:14'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:97-99`

**What**: normalize_repo_url rewrites a non-scheme, non-shorthand value matching is_bare_github_shorthand to https://github.com/{s} directly, with no scrub. is_bare_github_shorthand accepts `..` as either segment because ident_ok only requires bytes in [A-Za-z0-9._-], and `..` contains only allowed bytes. An adversarial package.json `repository = "../etc"` is rewritten to `https://github.com/../etc`, then rendered into About cards / markdown / HTML / JSON outputs and operator-facing logs.

**Why it matters**: Sister branches (github:, git://, git+*://) all route through scrub_path_segments per SEC-14 / TASK-1111, but this branch was added by PATTERN-1 / TASK-1060 without the same scrub. Downstream renderers and audit consumers capture the literal `..` traversal form even though browsers may collapse it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A new test normalize_bare_shorthand_strips_traversal asserts that normalize_repo_url('../etc') does not contain  and lands on https://github.com/etc (or collapses to https://github.com if every segment filters out).
- [x] #2 is_bare_github_shorthand is updated (or the branch routes through scrub_path_segments) so that segments equal to . or .. are rejected/dropped before the URL is synthesised; existing tests continue to pass.
<!-- AC:END -->
