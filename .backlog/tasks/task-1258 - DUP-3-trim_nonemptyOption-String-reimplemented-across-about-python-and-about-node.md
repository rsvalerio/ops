---
id: TASK-1258
title: >-
  DUP-3: trim_nonempty(Option<String>) reimplemented across about-python and
  about-node
status: To Do
assignee:
  - TASK-1265
created_date: '2026-05-08 13:02'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:360` and `extensions-node/about/src/package_json.rs:147`

**What**: Both about-python and about-node define identical `fn trim_nonempty(value: Option<String>) -> Option<String>` helpers with the same body. The Python `extract_urls` adds an inline trim+filter at line 352. Each future ERR-2 fix has to land in three places.

**Why it matters**: Drift risk already realised via TASK-0563/0704/0813/0814 landing per-site at different times. Lifting to ops_about (next to manifest_io/workspace/identity helpers already shared) collapses the three copies onto one policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops_about exports a single trim_nonempty consumed by both about-node and about-python
- [ ] #2 Both crates drop their local copy
- [ ] #3 Inline trim+filter chain in pick_url rewritten in terms of the shared helper
<!-- AC:END -->
