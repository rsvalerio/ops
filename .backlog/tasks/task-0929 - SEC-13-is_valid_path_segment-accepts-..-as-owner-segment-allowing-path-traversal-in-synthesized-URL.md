---
id: TASK-0929
title: >-
  SEC-13: is_valid_path_segment accepts '..' as owner segment, allowing
  path-traversal in synthesized URL
status: Done
assignee: []
created_date: '2026-05-02 15:33'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:158`

**What**: `is_valid_path_segment` allows segments composed entirely of `.` characters, so `parse_remote_url("https://github.com/../etc.git")` succeeds with `owner = ".."`, `repo = "etc"`, and `url = "https://github.com/../etc"`. The reconstructed URL is then surfaced via `git_info.remote_url` into about cards, JSON output, and downstream renderers as a clickable link. Browsers will collapse `../` away (landing on `https://github.com/etc`), but downstream tools that consume the JSON literally — backups, audit logs, tickets, mirrors — capture the traversal form. The hostile owner can come straight from `.git/config`, which is parsed automatically by every `ops about` run inside any cloned repo.

**Why it matters**: SEC-11/SEC-13 hardening in TASK-0782/TASK-0664/TASK-0465 closed the host and the smuggled-char surfaces but left the dot-only segment shape open. Users who paste the rendered URL into a browser get redirected silently to a different repo than the one they thought they were inspecting. Low severity because the misdirection is contained to the displayed link, but the fix is a one-line `if rest.iter().all(|b| *b == b'.')` reject and aligns owner/repo with the host validator that already rejects empty labels (TASK-0782).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 is_valid_path_segment rejects any segment whose non-~-prefix bytes are entirely '.' ('.', '..', '...', etc.).
- [x] #2 New unit test pins that parse_remote_url("https://github.com/../etc.git"), parse_remote_url("https://gitlab.com/group/../repo.git"), and parse_remote_url("https://github.com/owner/..") all return None.
- [ ] #3 Existing positive tests for legitimate '.'-containing names (e.g. host git.example.com unaffected; owner like my.lib, repo like lib.rs) continue to pass.
<!-- AC:END -->
