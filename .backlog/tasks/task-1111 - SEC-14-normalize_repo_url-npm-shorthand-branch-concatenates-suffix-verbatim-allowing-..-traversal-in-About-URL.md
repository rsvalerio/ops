---
id: TASK-1111
title: >-
  SEC-14: normalize_repo_url npm-shorthand branch concatenates suffix verbatim,
  allowing ../ traversal in About URL
status: Done
assignee: []
created_date: '2026-05-07 21:48'
updated_date: '2026-05-08 12:00'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:18-29` (`normalize_repo_url` HOST_PREFIXES branch)

**What**: For the `github:`, `gitlab:`, `bitbucket:` npm-shorthand prefixes, `normalize_repo_url` returns `format!("https://{host}/{rest}")` with **no validation of `rest`**. An adversarial `package.json` carrying `"repository": "github:../../etc/passwd"` produces `https://github.com/../../etc/passwd`. Same root cause class as SEC-14 / TASK-0811 (which fixed traversal in `append_tree_directory`'s `directory` field): a path component flows into a rendered URL surface (About cards, markdown, HTML) without segment scrubbing.

The `git+` and `git://` branches share the same shape — `format!("https://{}", rest.trim_end_matches(".git"))` on `git://owner/../../etc/passwd` produces an identical traversal-shaped URL.

**Why it matters**: The About card / identity JSON gets rendered into terminal output, markdown, HTML, and log lines. A `..`-bearing URL is a real surface for path-shape attacks against any consumer that resolves the URL or pattern-matches its path component. SEC-14 / TASK-0811 already accepted this threat model for the `directory` field; the shorthand branch is the same surface with the same fix shape (`split('/')` filter `..` / `.` / empty / leading slashes).

**Suggested fix**: Apply the same `split('/').filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..").collect()` segment-scrub used in `append_tree_directory` to the `rest` portion of every shorthand / scheme rewrite, OR reject the URL outright (return the raw input) when the suffix contains a `..` segment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 github:/gitlab:/bitbucket:/git+/git:// branches reject or sanitise .. segments before interpolation
- [x] #2 Behaviour parity with append_tree_directory's segment scrub (TASK-0811 / SEC-14)
- [x] #3 Unit tests cover traversal shapes such as github:../../etc/passwd and git scheme equivalents
<!-- AC:END -->
