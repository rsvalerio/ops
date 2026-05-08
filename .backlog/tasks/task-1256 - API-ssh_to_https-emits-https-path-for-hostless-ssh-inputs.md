---
id: TASK-1256
title: 'API: ssh_to_https emits https:///<path> for hostless ssh:// inputs'
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 13:01'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:138`

**What**: `ssh_to_https("/path")` (e.g. from `ssh:///path` or `ssh://git@/path`) skips the scp-form rewrite because `split_once(':')` returns None and `is_numeric_port_prefix` is irrelevant. The function returns `"https:///path"` — a syntactically broken URL with empty authority — which then renders as a clickable link in About cards.

**Why it matters**: The function precondition (well-formed `ssh://[user@]host[:port|path]`) is violated by hostile or typoed `package.json` `repository.url` values; the result reaches operator-facing surfaces verbatim. Same operator-surface concern as TASK-1080 (control chars) and TASK-1111 (traversal).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hostless inputs return a deterministic safe shape (drop the field, or fall through to the trimmed verbatim string)
- [ ] #2 Unit test pinning ssh_to_https("/path") and ssh_to_https("git@:foo") outputs
- [ ] #3 About-card render path no longer surfaces https:/// URLs
<!-- AC:END -->
