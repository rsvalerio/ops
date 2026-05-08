---
id: TASK-1169
title: 'API: normalize_repo_url returns owned String even on identity passthrough'
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 07:46'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:35-101`

**What**: `normalize_repo_url(&str) -> String` always allocates. Unrecognised-input branch falls through to `s.trim_end_matches(\".git\").to_string()` (line 100); bare-shorthand passthrough also clones. Signature also makes it impossible for callers to detect \"no normalisation applied\" or short-circuit a downstream cache.

**Why it matters**: Minor performance and API-shape concern. Returning Cow<'_, str> would let parse_package_json skip the .to_string() clone in the no-change passthrough (every legitimate https://github.com/... URL hits this).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 normalize_repo_url returns Cow<'_, str>; identity passthroughs return Cow::Borrowed
- [ ] #2 Existing tests adapt by calling .as_ref() or .into_owned() at the assertion site
- [ ] #3 No behavioural change beyond fewer allocations on the hot path
<!-- AC:END -->
