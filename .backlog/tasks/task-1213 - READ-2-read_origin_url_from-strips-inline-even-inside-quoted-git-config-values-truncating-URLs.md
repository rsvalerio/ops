---
id: TASK-1213
title: >-
  READ-2: read_origin_url_from strips inline ; even inside quoted git-config
  values, truncating URLs
status: To Do
assignee:
  - TASK-1267
created_date: '2026-05-08 08:19'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:237-248`

**What**: strip_url_key uses `value.find(['#', ';']).unwrap_or(value.len())` to drop inline comments. The function-level comment correctly notes "Quoted values are not yet honoured" — but the consequence is that a real-world git-config line like `url = "https://example.com/path;tag=v1"` silently truncates at the first ;, producing `"https://example.com/path` (with a leading quote) and routes through RedactedUrl::redact which keeps it.

**Why it matters**: Real-world git-config emits quoted values for URLs containing shell metacharacters, especially in tools that template the config. Silent truncation is the worst of both worlds (no log, malformed downstream URL).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 strip_url_key recognises a leading double-quote and returns the body up to the matching closing quote, decoding escape rules already implemented in parse_section_header. Inline comments inside a quoted value pass through unchanged.
- [ ] #2 A new test origin_url_quoted_value_with_semicolon_round_trips writes a git-config containing a quoted URL with embedded ; and asserts read_origin_url_from returns the full URL (no truncation, no leading quote).
<!-- AC:END -->
