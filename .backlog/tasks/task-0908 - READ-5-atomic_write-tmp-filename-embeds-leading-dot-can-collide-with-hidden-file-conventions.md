---
id: TASK-0908
title: >-
  READ-5: atomic_write tmp filename embeds leading dot, can collide with
  hidden-file conventions
status: Triage
assignee: []
created_date: '2026-05-02 10:10'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:95`

**What**: The temp basename is composed as a hidden-file-prefixed name. If file_name itself starts with a dot (e.g. .ops.toml), the resulting `..ops.toml.tmp.…` is unusual; cleanup scripts and editor swap-file detectors can misclassify it; grep-based crash-recovery audits skip it.

**Why it matters**: Operational hygiene — leftover temp files from a crash are non-trivial to spot or clean because the naming pattern double-prefixes a dot.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Strip a leading dot from file_name before composing the tmp basename, OR adopt a uniform .ops_atomic_<id> prefix
- [ ] #2 Test asserts the tmp basename does not begin with two dots
<!-- AC:END -->
