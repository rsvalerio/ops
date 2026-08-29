---
id: TASK-2016
title: >-
  CL-3: rust-loc EXCLUDED_DIRS prunes at any depth while the tokei policy it
  cites now anchors to the scan root
status: Done
assignee:
  - TASK-2050
created_date: '2026-08-28 15:56'
updated_date: '2026-08-29 13:12'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:90-166`

**What**: `EXCLUDED_DIRS` is documented as "mirroring the
`TOKEI_DEFAULT_EXCLUDED` policy in the tokei extension", and `is_excluded_dir`
prunes any directory with a matching name at any depth except the walk root
(`entry.depth() == 0`). TASK-1974 has since changed the tokei side: its
exclusion list now prunes only **direct children of the scan root**
(`entry.depth() != 1` returns false in `extensions/tokei/src/lib.rs`), because
matching `build/` or `dist/` at arbitrary depth silently dropped real source --
a Python package `pkg/build/`, a JS source folder `src/dist/`. The two
extensions therefore no longer implement the same policy, while `rust-loc`'s
doc comment still says they do.

`rust-loc` only counts `.rs` files, so the concrete exposure is narrower than
tokei's -- a Rust source directory named `target` or `.git` below the root --
but the divergence between the code and the comment that points at the other
crate is the reviewable defect either way.

**Why it matters**: CL-3 -- the doc states an assumption the implementation no
longer holds, and it does so by reference to a second crate that has moved. The
next author reading either constant cannot tell which anchoring is current.

**Origin**: discovered during TASK-2012 while fixing TASK-1974.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rust-loc's exclusion anchoring either matches the root-anchored tokei policy or its doc comment states the deliberate difference instead of claiming to mirror it
- [x] #2 A test covers the chosen behaviour for a directory with an excluded name nested below the scan root
<!-- AC:END -->
