---
id: TASK-2395
title: 'DUP-2: read_origin_url and read_head_branch duplicate the bounded open/take/read_to_end/cap-check sequence'
status: Triage
assignee: []
created_date: '2026-10-04 14:15'
labels:
  - code-review-rust
  - DUP
dependencies: []
modified_files:
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-2:extensions/git/src/config.rs:read_origin_url'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:read_origin_url`, `extensions/git/src/config.rs:read_head_branch`

**What**: Both functions repeat the same ~30-line sequence: open (NotFound -> None, other error -> warn + None), `take(cap + 1).read_to_end`, warn on I/O error, `u64::try_from(len) > cap` check with SEC-33 warn. Only the path, cap and message text differ.

**Why it matters**: A fix to the capped-read policy (e.g. error handling, cap semantics) must be made twice and can diverge; a third `.git` file reader would copy it again.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper (path, cap, label) -> Option<Vec<u8>> implements the capped read and both callers use it
- [ ] #2 Existing cap/NotFound/oversize tests still pass
<!-- AC:END -->
