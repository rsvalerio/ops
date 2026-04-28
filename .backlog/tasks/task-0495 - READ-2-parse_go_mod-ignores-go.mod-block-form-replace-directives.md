---
id: TASK-0495
title: 'READ-2: parse_go_mod ignores go.mod block-form replace directives'
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 06:09'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - read
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:112`

**What**: parse_go_mod only matches single-line `replace foo => ./bar` and silently ignores the `replace ( ... )` block form, which is the more common style for monorepos with multiple local replaces. Lines inside a `replace (` block do not start with `replace ` so strip_prefix returns None and the local replace is lost from module_count.

**Why it matters**: Underreports module_count for any Go project that groups its replace directives in a block — exactly the monorepos this code targets. The behavior is silently wrong, not a parse error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_go_mod recognizes replace ( block boundaries and parses each <from> => <to> line within it
- [ ] #2 Local-only filtering (./ prefix) still applies to block entries
- [ ] #3 Test added covering replace-block input matching a real monorepo layout
<!-- AC:END -->
