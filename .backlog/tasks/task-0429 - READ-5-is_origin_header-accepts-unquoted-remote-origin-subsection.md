---
id: TASK-0429
title: 'READ-5: is_origin_header accepts unquoted [remote origin] subsection'
status: Done
assignee:
  - TASK-0535
created_date: '2026-04-28 04:42'
updated_date: '2026-04-28 13:44'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:80`

**What**: `subsection == "\"origin\"" || subsection == "origin"` accepts both `[remote "origin"]` (canonical, what git writes) and `[remote origin]` (which git itself rejects as malformed). The forgiving form can match sections that git would not honor.

**Why it matters**: The parser may report an "origin URL" for a config layout that the actual git binary ignores, producing a GitInfo.remote_url that diverges from what `git remote -v` shows in the same checkout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Drop the subsection == "origin" branch (require the quoted form)
- [x] #2 Add a test demonstrating [remote origin] (no quotes) is not treated as origin
- [x] #3 Existing canonical [remote "origin"] test stays green
<!-- AC:END -->
