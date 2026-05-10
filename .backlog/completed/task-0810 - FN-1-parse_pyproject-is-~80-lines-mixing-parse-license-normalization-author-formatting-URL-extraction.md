---
id: TASK-0810
title: >-
  FN-1: parse_pyproject is ~80 lines mixing parse, license normalization, author
  formatting, URL extraction
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:02'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:169-249`

**What**: One function deserializes pyproject.toml, applies trim filters, normalises the three-shape LicenseField, formats authors, and runs pick_url for two link kinds. Each subtask is its own abstraction level.

**Why it matters**: Sister parsers parse_pom_xml (TASK-0319) and parse_go_mod (compute_module_count extracted) already split. This file is the remaining outlier; complexity makes future PEP 621 additions risky.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract license, authors, urls into named helpers (e.g. normalize_license, format_authors, extract_urls)
- [ ] #2 Resulting parse_pyproject body <=30 lines
- [ ] #3 Behaviour preserved (existing tests green; trim/empty-drop semantics unchanged)
<!-- AC:END -->
