---
id: TASK-1249
title: 'CONC-7: capture_path_index_from stores file_name case-sensitively on Windows'
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 14:56'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:182`

**What**: `capture_path_index_from` inserts `entry.file_name()` into `HashSet<OsString>` and `is_in_path_index` does case-sensitive `.contains` lookups. On Windows the filesystem is case-insensitive (tokei.EXE, Tokei.exe, tokei.exe all refer to the same file) but the index is case-sensitive, so a probe for "tokei" misses an on-disk binary named Tokei.exe. The PATHEXT loop in `is_in_path_index` only covers suffix variation, not basename casing.

**Why it matters**: False-negative install probes on Windows: cargo-installed binaries with mixed-case names look uninstalled, drive a redundant install on every `ops about`, and cycle indefinitely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Lowercase index keys and lookup name on Windows in a small IndexKey wrapper
- [ ] #2 Leave Unix path unchanged
- [ ] #3 Unit test pinning tokei matches Tokei.EXE under cfg(windows)
<!-- AC:END -->
