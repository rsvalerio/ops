---
id: TASK-1861
title: >-
  SEC-11: the ALLOWED_SCHEMES gate is bypassed for single-colon URIs — file:/…,
  javascript:… and https:/typo parse as scp remotes with a fabricated host
status: Done
assignee:
  - TASK-2007
created_date: '2026-08-27 15:29'
updated_date: '2026-08-28 23:26'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/git/src/remote.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:76-104` (`split_scheme_host_and_path`), consumed by `extensions/git/src/provider.rs:55`

**What**: `split_scheme_host_and_path` only consults `ALLOWED_SCHEMES` on the `://` branch. Any value that does **not** contain `://` falls into the scp branch, which treats everything before the first `:` as the host and everything after as the path, with no scheme check at all:

- `file:/srv/git/o/repo.git` → `host = "file"`, `owner = "srv/git/o"`, `repo = "repo"`, `url = "ssh://file/srv/git/o/repo"`
- `javascript:evil/repo` → `host = "javascript"`, `url = "ssh://javascript/evil/repo"`
- `https:/github.com/o/r` (single-slash typo) → `host = "https"`, `owner = "github.com/o"`, `repo = "r"`

The `ALLOWED_SCHEMES` doc comment at `remote.rs:67-70` states: "`file://`, `javascript:`, and other custom schemes are rejected to keep attacker-influenced git config values from producing unsafe URLs downstream." That guarantee is false for exactly the single-colon form the comment names (`javascript:`). The test suite only pins the `://` forms (`file_scheme_is_rejected`, `rejects_unknown_scheme`), so the gap is invisible.

**Why it matters**: `git_info.host` / `owner` / `repo` / `remote_url` are the canonical repository-identity fields every other extension trusts (`extensions/about/src/identity.rs:149`, `extensions-rust/about/src/identity/resolver.rs:40`). A hostile or merely unusual `.git/config` yields a confidently-wrong remote: `file:/…` local remotes are silently re-advertised as `ssh://file/…`, which is the same transport-misattribution class TASK-1237 was filed and fixed for on the `://` path — audit/policy code that distinguishes scheme sees a fabricated ssh remote to a host named `file`. A value the parser should reject fails **open** into operator-facing surfaces instead of dropping to `None` (the fail-closed posture SEC-13 / TASK-1151 established at `provider.rs:87`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 split_scheme_host_and_path rejects any input whose pre-colon segment is a URI scheme not in ALLOWED_SCHEMES when the value is not a genuine scp-style host:path remote (i.e. the scp branch must not silently accept file:, javascript:, ftp:, mailto:, or a single-slash https:/ typo)
- [x] #2 parse_remote_url returns None for 'file:/srv/git/o/repo.git', 'javascript:evil/repo', 'ftp:host/o/r', and 'https:/github.com/o/r'; genuine scp remotes ('git@github.com:o/r.git' and the already-redacted 'github.com:o/r.git') still parse to ssh://
- [x] #3 the ALLOWED_SCHEMES doc comment matches the implemented behaviour, or is amended to state precisely which forms are gated
- [x] #4 regression tests cover each rejected single-colon form and both accepted scp forms
<!-- AC:END -->
