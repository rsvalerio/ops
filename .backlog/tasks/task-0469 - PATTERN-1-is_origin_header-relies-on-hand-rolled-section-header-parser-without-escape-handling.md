---
id: TASK-0469
title: >-
  PATTERN-1: is_origin_header relies on hand-rolled section-header parser
  without escape handling
status: Done
assignee:
  - TASK-0535
created_date: '2026-04-28 05:46'
updated_date: '2026-04-28 13:44'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:80`

**What**: is_origin_header accepts subsection forms `"origin"` and `origin` via two literal `==` comparisons after splitting on whitespace. git-config supports backslash-escaped quotes within subsection names (`\\"`, `\\\\`); a remote like `[remote "or\\"igin"]` is technically legal and the matcher silently fails.

**Why it matters**: TASK-0429 covers the unquoted case but the broader issue is the hand-rolled parser. Distinct concern: if read_upstream_url etc. is added, the same brittle shape will accumulate one-off checks at every new call site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract a private fn parse_section_header(line: &str) -> Option<(&str, Option<String>)> that decodes git-config quoting (drops surrounding quotes and unescapes \\" and \\\\)
- [x] #2 is_origin_header rewritten in terms of the helper; gains a test for [remote "or\\"igin"] (rejected) plus [remote "origin"] with leading/trailing whitespace inside the quotes
<!-- AC:END -->
