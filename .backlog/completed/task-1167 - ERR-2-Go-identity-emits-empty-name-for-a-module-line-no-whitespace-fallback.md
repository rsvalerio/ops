---
id: TASK-1167
title: >-
  ERR-2: Go identity emits empty name for a 'module ""' line, no whitespace
  fallback
status: Done
assignee:
  - TASK-1267
created_date: '2026-05-08 07:45'
updated_date: '2026-05-09 14:39'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:40-41`

**What**: `module ` lines parse via `rest.trim().to_string()` with no empty-filter, so `\"module \\n\"` (or `\"module    \"`) yields `Some(GoMod { module: \"\".to_string(), ... })`. `parse_go_mod` in lib.rs:114-119 promotes that empty string into a GoMod, and lib.rs:65-68 takes `\"\".rsplit('/').next() == Some(\"\")`, surfacing an empty name. Sister stacks (Node `package_json::trim_nonempty`, Python `trim_nonempty` at lib.rs:360-364) drop whitespace-only fields.

**Why it matters**: Cross-stack inconsistency. Node and Python fall back to the directory name; Go renders an About card with a blank identity field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_go_mod (or go_mod::parse) drops a whitespace-only module value to None
- [x] #2 Test writes 'module    \n\ngo 1.22\n' and asserts rendered identity falls back to directory name like Node/Python
<!-- AC:END -->
