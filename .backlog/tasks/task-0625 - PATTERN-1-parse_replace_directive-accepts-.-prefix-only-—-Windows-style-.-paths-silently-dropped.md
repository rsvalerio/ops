---
id: TASK-0625
title: >-
  PATTERN-1: parse_replace_directive accepts ./ prefix only — Windows-style .\\
  paths silently dropped
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:21'
updated_date: '2026-04-29 12:10'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:70`

**What**: parse_replace_directive (go_mod.rs:70-78) treats only `./...` targets as local replaces. Go on Windows accepts `.\sub` as valid relative replace path. Current parser silently treats `.\sub` as non-local, dropping it from local_replaces. Same caveat on go_work::parse_use_dirs (go_work.rs:9-54).

**Why it matters**: Silent drop of legitimate (if uncommon) input is exactly the PATTERN-1 smell — partial-input handler that looks total. Documenting limitation (or accepting both prefixes) makes the contract explicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_replace_directive accepts both ./ and .\\ (or docstring documents Unix-only contract)
- [ ] #2 go_work::parse_use_dirs follows same convention
- [ ] #3 Test pins chosen behaviour
<!-- AC:END -->
