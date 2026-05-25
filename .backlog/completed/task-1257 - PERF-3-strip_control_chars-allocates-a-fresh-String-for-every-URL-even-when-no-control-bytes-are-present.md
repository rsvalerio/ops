---
id: TASK-1257
title: >-
  PERF-3: strip_control_chars allocates a fresh String for every URL even when
  no control bytes are present
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 13:01'
updated_date: '2026-05-09 11:23'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:18`

**What**: `strip_control_chars` walks every char and collects into a fresh String, then `normalize_repo_url` immediately trims and re-walks the result. The common case (well-formed `https://github.com/...`) has zero control bytes; the allocation+copy is pure overhead per `parse_package_json` call.

**Why it matters**: `normalize_repo_url` runs once per package.json per About invocation, and in a workspace with N members the units provider also routes each member's repository field through it. Cheap fast-path: if `raw.bytes().all(|b| !b.is_ascii_control() && b != 0x7f)`, return Cow::Borrowed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Hot-path returns Cow<str> (or short-circuits the allocation) when no control byte is present
- [x] #2 Bench or microtest shows zero String allocation on a clean URL
- [x] #3 Regression tests for the strip behaviour stay green
<!-- AC:END -->
