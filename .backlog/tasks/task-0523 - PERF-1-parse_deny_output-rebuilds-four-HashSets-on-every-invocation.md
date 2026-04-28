---
id: TASK-0523
title: 'PERF-1: parse_deny_output rebuilds four HashSets on every invocation'
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 18:01'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:302`

**What**: ADVISORY_CODES, LICENSE_CODES, BAN_CODES, SOURCE_CODES are `&[&str]` constants, but parse_deny_output rebuilds HashSet<&str> from each on every call. With small constant arrays (~5 entries), a linear .contains is faster than building a HashSet for one or two diagnostic lookups.

**Why it matters**: Either keep as slices and switch lookups to .contains(&code), or move HashSets behind OnceLock. Building four hashmaps per invocation just to query a 5-element table is anti-idiomatic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Slice .contains lookup OR OnceLock-cached HashSets
- [x] #2 Existing parse_deny_output tests still pass
<!-- AC:END -->
