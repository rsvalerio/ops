---
id: TASK-1162
title: >-
  TEST-12: typed_manifest_cache poison tests duplicate ~80 lines of capture
  harness
status: Done
assignee:
  - TASK-1265
created_date: '2026-05-08 07:45'
updated_date: '2026-05-09 13:45'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:657`

**What**: `typed_manifest_cache_recovers_from_poison_with_warn` (657-736) and `typed_manifest_cache_second_poison_still_logs` (744-810) share ~80 lines verbatim: same BufWriter shim, same poison-thread spawn, same subscriber wiring, same ctx/dir setup. They differ only in number of poison cycles and which log assertion runs.

**Why it matters**: TEST-12 says \"no redundant tests: identical logic, same paths, copy-paste with trivial differences\". A future change to the warn payload becomes a 4-place edit (this file × 2 + identical capture harness in coverage_provider × 1 + metadata ingestor × 1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Parameterise poison count via assert_poison_warn_after(n_cycles: usize) helper returning captured logs
- [x] #2 Keep two distinct test names so failure mode is attributable but each body becomes a 5-line call to the helper
<!-- AC:END -->
