---
id: TASK-0685
title: >-
  READ-2: pyproject pick_url precedence list contains kebab/space duplicates
  that normalize to same key
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 09:46'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:218-230`

**What**: `pick_url` looks up `"source code"` and `"source-code"` (and `"home-page"` vs `"home page"`) as separate candidates, but `normalize_url_key` already replaces `-` with space, so each kebab/space pair is the same normalized key — the duplicate slot in the list is dead.

**Why it matters**: Future maintainers reading the list assume two distinct lookups happen; an actual extra spelling (e.g. "sources") is more likely to be added by editing one of the apparent duplicates instead of inserting a real new variant. The current behaviour also makes the precedence list shorter than it appears.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The keys array drops one of each kebab/space duplicate (keep the canonical kebab form), or pick_url is documented to receive pre-normalised keys
- [x] #2 A test exercises a Source-Code (mixed case kebab) key and pins the order between repository and source taking precedence over source-code
<!-- AC:END -->
