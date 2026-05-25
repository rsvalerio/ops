---
id: TASK-0964
title: >-
  ERR-2: pyproject pick_url returns whitespace-only URL as Some("  ") instead of
  dropping
status: Done
assignee: []
created_date: '2026-05-04 21:47'
updated_date: '2026-05-04 22:54'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:250-288` (pick_url)

**What**: `pick_url` filters via `s.is_empty()` but does not `trim()` first. A `[project.urls]` entry like `Homepage = "  "` returns `Some("  ")`, which then renders as a blank About bullet — exactly the failure mode `trim_nonempty` was introduced to prevent for `name`/`license`/`requires_python`/authors (TASK-0704).

**Why it matters**: Inconsistent with the project-wide ERR-2 trim+drop-empty policy already applied to every other pyproject string field. A whitespace-only URL is a real shape (operators paste-and-clear).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pick_url trims the value before the empty check (e.g. .filter(|s| !s.trim().is_empty())) and returns the trimmed form
- [ ] #2 Test asserts a Homepage with whitespace-only value resolves to homepage = None, not an empty bullet
<!-- AC:END -->
