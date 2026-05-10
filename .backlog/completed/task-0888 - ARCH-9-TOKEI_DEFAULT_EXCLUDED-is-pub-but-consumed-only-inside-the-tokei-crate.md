---
id: TASK-0888
title: 'ARCH-9: TOKEI_DEFAULT_EXCLUDED is pub but consumed only inside the tokei crate'
status: Done
assignee: []
created_date: '2026-05-02 09:38'
updated_date: '2026-05-02 11:07'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:100`

**What**: `pub const TOKEI_DEFAULT_EXCLUDED: &[&str] = &[...]` is exposed as part of the tokei crate public API but every reference is inside the same crate (only `collect_tokei` reads it, also in `lib.rs`). Marking it `pub` widens the surface unnecessarily and freezes the exclusion list as a public contract that consumers may start to depend on.

**Why it matters**: ARCH-9 / minimum-public-surface — in Rust, `pub` is a deliberate API commitment. A future change to the exclusion list (adding `dist-newstyle`, dropping `venv` for a `*.venv*` glob, etc.) becomes a SemVer-relevant breaking change once any external consumer matches on the exact slice contents. Demote to `pub(crate)` (or private with a `cfg(test)` accessor for the lone test that covers it).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TOKEI_DEFAULT_EXCLUDED demoted to pub(crate) or private
- [ ] #2 no out-of-crate consumers in the workspace reference the constant
<!-- AC:END -->
