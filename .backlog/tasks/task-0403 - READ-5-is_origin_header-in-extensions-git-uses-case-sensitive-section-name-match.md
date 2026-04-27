---
id: TASK-0403
title: >-
  READ-5: is_origin_header in extensions/git uses case-sensitive section name
  match
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:52'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:70-77`

**What**: `is_origin_header` compares section name with `section == "remote"` (case-sensitive). Per git-config(1), section names are case-insensitive; real-world configs written by other tooling may use `[REMOTE "origin"]` or `[Remote "origin"]`. The current matcher silently misses such sections, causing `read_origin_url_from` to return `None` even when a valid origin remote is present. The lowercase-key fallback (URL/url) shows the parser already aims for case-insensitivity, but the section-name leg is inconsistent.

**Why it matters**: Drops valid origin URLs on inputs git itself accepts. The downstream effect is the `git_info` data provider returns `host=None, owner=None, repo=None, remote_url=None` for repositories with non-lowercase section headers, silently degrading every dependent surface (about cards, identity providers).

<!-- scan confidence: high; behavior verified by reading the parser -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 section-name comparison is case-insensitive (use eq_ignore_ascii_case)
- [ ] #2 regression test exercises an [REMOTE "origin"] config and asserts URL is parsed
<!-- AC:END -->
