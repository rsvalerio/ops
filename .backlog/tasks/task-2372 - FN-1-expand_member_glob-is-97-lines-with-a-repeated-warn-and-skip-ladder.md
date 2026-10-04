---
id: TASK-2372
title: 'FN-1: expand_member_glob is ~97 lines with a repeated warn-and-skip ladder'
status: Triage
assignee: []
created_date: '2026-10-04 14:11'
labels:
  - code-review-rust
  - FN
dependencies: []
modified_files:
  - extensions-rust/about/src/members.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/about/src/members.rs:expand_member_glob'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs:349`

**What**: `expand_member_glob` (lines 349-~446, ~97 lines) resolves the root, reads the dir, then per entry does a read error branch, canonicalize, containment check, is_dir/Cargo.toml probe, strip_prefix and UTF-8 conversion, each with its own `tracing::warn!` + skip. Per-entry work sits inside a `for` > `match` arm, mixing IO, security containment and logging at one level.

**Why it matters**: Exceeds FN-1 (50 lines). The SEC-14 containment decision is buried in the loop body and cannot be tested without a real directory tree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 expand_member_glob is <= 50 lines; per-entry resolution (canonicalize + containment + manifest probe + relpath) is a named helper returning Option<String>
- [ ] #2 Existing members tests pass unchanged
<!-- AC:END -->
