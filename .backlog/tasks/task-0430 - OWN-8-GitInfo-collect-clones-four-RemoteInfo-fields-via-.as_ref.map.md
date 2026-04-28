---
id: TASK-0430
title: 'OWN-8: GitInfo::collect clones four RemoteInfo fields via .as_ref().map()'
status: Done
assignee:
  - TASK-0535
created_date: '2026-04-28 04:42'
updated_date: '2026-04-28 13:45'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/provider.rs:50-61`

**What**: `parsed.as_ref().map(|r| r.host.clone())`, `r.owner.clone()`, `r.repo.clone()`, `r.url.clone()` — `parsed` is otherwise unused after this block, so the `as_ref().map(... .clone())` pattern clones four Strings that could be moved out by destructuring `parsed` once.

**Why it matters**: Cold path (one call per about render), so impact is small, but the pattern is the textbook "clone to satisfy the borrow checker" smell OWN-8 calls out and a reader has to verify all four sites independently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace the four .as_ref().map(... .clone()) calls with a single match on parsed that destructures RemoteInfo into host/owner/repo/url (or equivalent)
- [x] #2 Behavior unchanged: existing collect_full_github_remote / collect_unparseable_remote_* tests still pass
<!-- AC:END -->
