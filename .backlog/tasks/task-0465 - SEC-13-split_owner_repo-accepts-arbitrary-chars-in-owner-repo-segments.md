---
id: TASK-0465
title: 'SEC-13: split_owner_repo accepts arbitrary chars in owner/repo segments'
status: To Do
assignee:
  - TASK-0535
created_date: '2026-04-28 05:46'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:103`

**What**: split_owner_repo takes the last two non-empty `/`-separated segments and returns them verbatim. is_valid_host enforces a strict reg-name allowlist on the host slot, but owner/repo get only "non-empty after splitting". `https://github.com/foo\u{0007}/bar` parses successfully and RemoteInfo.owner = "foo\u{0007}" flows into the Serialize'd JSON and the synthetic url field. Backslashes, spaces, control chars, embedded `?`/`#`/`@` fragments all pass.

**Why it matters**: Struct doc says "no credentials, no .git suffix, normalized https URL", but the URL is reconstructed from unsanitised owner/repo so a malformed origin URL produces an output that *looks* normalized while carrying smuggled bytes. Distinct from TASK-0428 which only covered split_owner_repo charset for valid hosts; this targets the reconstructed URL output specifically.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 split_owner_repo (or parse_remote_url) rejects owner/repo segments containing characters outside [A-Za-z0-9._~/-] plus a small whitelist for sourcehut-style ~user
- [ ] #2 Tests for parse_remote_url("https://github.com/foo\\u{0007}/bar") and similar control-char inputs return None
<!-- AC:END -->
