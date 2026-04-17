---
id: TASK-0031
title: 'CD-3: capitalize() function duplicated in 2 crates'
status: Done
assignee: []
created_date: '2026-04-14 19:35'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-1
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: extensions/about/src/lib.rs:280-286, crates/cli/src/extension_cmd.rs:65-71
**Anchor**: fn capitalize
**Impact**: Identical 7-line capitalize() function appears in 2 separate crates. Both implementations: take &str, extract first char, uppercase it, concatenate with remainder. Currently only 2 occurrences, but both are in separately maintained crates with no shared dependency path for this utility.

Fix: move to ops_core as a text utility (e.g., ops_core::text::capitalize) or add to an existing utility location. Minor fix since the function is small, but it prevents drift.

DUP-1: identical code block (7 lines).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 capitalize() is defined once and imported where needed
<!-- AC:END -->
