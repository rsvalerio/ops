---
id: TASK-1621
title: 'API-5: ops-git public pure return-only functions lack #[must_use]'
status: Done
assignee:
  - TASK-1639
created_date: '2026-05-22 07:04'
updated_date: '2026-05-22 13:27'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:41`, `extensions/git/src/config.rs:142`, `extensions/git/src/config.rs:230`, `extensions/git/src/config.rs:469`

**What**: Several public functions in `ops-git` are pure or near-pure functions whose entire purpose is the returned value, yet they lack `#[must_use]`:

- `remote::parse_remote_url(raw: &str) -> Option<RemoteInfo>` — pure parser; sibling `RedactedUrl::redact` and `resolve_repository_with_git_fallback` are already `#[must_use]`.
- `config::read_origin_url_from(content: &str) -> Option<RedactedUrl>` — pure scanner over a `&str`; the returned `RedactedUrl` carries the SEC-13 redaction invariant that callers must not silently discard.
- `config::read_origin_url(git_dir: &Path) -> Option<RedactedUrl>` — observable side effects are only `tracing::warn!` events; the `Option<RedactedUrl>` is the load-bearing output.
- `config::read_head_branch(git_dir: &Path) -> Option<String>` — same shape as above.

**Why it matters**: API-5 (rules-structure). The crate already applies `#[must_use]` selectively (`RedactedUrl::redact`, `resolve_repository_with_git_fallback`, `RedactedUrl::as_str`, `RedactedUrl::into_string`), so the inconsistency is the signal: a caller who writes `read_origin_url_from(cfg);` or `parse_remote_url(raw);` and forgets to bind the result gets no lint, even though dropping the redacted URL is exactly the kind of mistake the `RedactedUrl` newtype was designed to make hard. Adding `#[must_use]` aligns these with the existing convention and gives the compiler a chance to catch silent drops.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_remote_url is annotated #[must_use]
- [ ] #2 read_origin_url, read_origin_url_from, and read_head_branch are annotated #[must_use]
- [ ] #3 cargo clippy -p ops-git passes without new warnings
<!-- AC:END -->
