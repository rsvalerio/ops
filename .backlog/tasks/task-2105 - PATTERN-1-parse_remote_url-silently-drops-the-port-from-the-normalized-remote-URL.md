---
id: TASK-2105
title: >-
  PATTERN-1: parse_remote_url silently drops the port from the normalized remote
  URL
status: To Do
assignee:
  - TASK-2243
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions/git/src/remote.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:171-176` (`split_scheme_host_and_path`), consumed at `extensions/git/src/remote.rs:73`

**What**: On the `scheme://` branch the authority is split as `let host = host_part.split(':').next()?;` — the port is discarded and never re-attached when `url` is rebuilt as `format!("{scheme}://{host}/{owner}/{repo}")`. `ssh://git@git.example.com:2222/o/r.git` therefore yields `RemoteInfo { host: "git.example.com", url: "ssh://git.example.com/o/r" }`, an URL that names a *different* endpoint (port 22) than the configured remote. The same applies to `https://gitea.internal:8443/o/r.git` -> `https://gitea.internal/o/r`. The current behaviour is pinned by the test `ssh_scheme_with_port` (remote.rs:400-406), so it reads as intentional, but nothing in the `RemoteInfo.url` invariant doc mentions dropping the port.

**Why it matters**: This is the same class of misattribution PATTERN-1 / TASK-1237 fixed for the scheme. `git_info.remote_url` is documented as a "normalized origin remote URL, preserving the input scheme" and flows into About cards, provider JSON, audit trails and mirrors. Self-hosted forges on non-default ports are common; a consumer that copies `remote_url` to clone, link, or record provenance gets an endpoint that either fails or — worse — resolves to a different service on the default port. `git_info.host` loses the port too, so a consumer cannot reconstruct it.

<!-- Reviewer note: either preserve the port in `host`/`url` (validating it as digits, <= 65535, and keeping `is_valid_host` applied to the hostname only), or, matching the SEC-13 / TASK-1151 fail-closed posture, reject ported remotes outright rather than emitting a URL that points elsewhere. Whichever is chosen, state it in the `RemoteInfo.url` invariant doc. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_remote_url either preserves an explicit port in RemoteInfo.host/url or rejects ported remotes; the chosen behaviour is documented in the RemoteInfo.url invariant and the git_info schema description
- [ ] #2 Tests cover ssh://host:2222/o/r, https://host:8443/o/r, and an invalid port (non-numeric, out of range)
- [ ] #3 ssh_scheme_with_port is updated so it no longer pins the lossy behaviour
<!-- AC:END -->
