---
id: TASK-0483
title: 'READ-4: pick_url uses hardcoded case-sensitive PEP 621 candidate keys'
status: Done
assignee:
  - TASK-0532
created_date: '2026-04-28 05:48'
updated_date: '2026-04-28 15:44'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-python/about/src/lib.rs:207-218

**What**: pick_url uses a hardcoded case-sensitive list of candidate keys against a BTreeMap<String, String>. PEP 621 places no constraints on [project.urls] key casing, so common variants like HomePage, Home Page, homepage (with trailing space), repo, Code, source-code are missed silently. The current list also includes 'Source Code' (with a space) but not 'source-code' or 'sourceCode'.

**Why it matters**: The fallback to git remote masks the missed repository URL only when .git/config is present; otherwise the about card repository is empty even though pyproject.toml declared one. There is no diagnostic that the URL was found-but-unmatched.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Lowercase-and-trim incoming keys at parse time (or do a case-insensitive lookup) and broaden the candidate list to include kebab-case spellings (source-code, home-page)
- [ ] #2 Add a unit test using [project.urls] with homepage (lowercase) and source-code keys and assert both are picked up
<!-- AC:END -->
