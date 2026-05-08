---
id: TASK-1102
title: >-
  SEC-2: RedactedUrl preserves control chars; raw newlines/ANSI from .git/config
  flow into JSON and About cards
status: Done
assignee: []
created_date: '2026-05-07 21:33'
updated_date: '2026-05-08 00:00'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:184-207` (consumed at `extensions/git/src/provider.rs:78`)

**What**: `redact_userinfo` strips a `user[:password]@` prefix but preserves every other byte verbatim. A `.git/config` line `url = https://host/repo\u{1b}[31m\nFAKE-LOG` flows through `read_origin_url_from` → `RedactedUrl` and, when `parse_remote_url` rejects the shape, the provider falls back at `provider.rs:78` to `remote_url: Some(raw.into_string())`. The raw string lands in serialised git_info JSON and is rendered into About card text and logs without sanitisation.

**Why it matters**: Defeats the same control-char hardening already applied to log fields elsewhere (TASK-0937, TASK-0974). Different surface from TASK-1080 (extensions-node).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 RedactedUrl::redact rejects (or strips) values containing ASCII control bytes (\x00..\x1f, \x7f) and returns None / a sanitised marker the provider treats as no remote
- [x] #2 GitInfo::collect fallback path either drops remote_url or surfaces a sanitised value when the input contained control chars
- [x] #3 Regression test: a config with url = https://host/repo\u{1b}[31m\nfake produces an info.remote_url that contains neither \n nor \u{1b}
<!-- AC:END -->
