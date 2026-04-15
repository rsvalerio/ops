---
id: TASK-0039
title: >-
  CQ-8: about-rust lib.rs centralizes 1588 lines of tests — violates ARCH-8 thin
  entry point
status: Done
assignee: []
created_date: '2026-04-14 20:14'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - ARCH-8
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/about/src/lib.rs is 1635 lines but only 47 lines are production code (module declarations, constants, macro invocation). The remaining ~1588 lines are a single #[cfg(test)] mod tests block containing 101+ test functions for all submodules (cards, format, query, text_util, identity). The submodules themselves have 0 tests (except dashboard with 8). This violates ARCH-8: lib.rs should be a thin entry point. Tests should live next to the code they cover per project convention (#[cfg(test)] mod tests in the same file).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests moved from lib.rs into respective submodule files (cards.rs, format.rs, query.rs, text_util.rs, identity.rs) as #[cfg(test)] mod tests blocks
- [ ] #2 lib.rs contains only module declarations, re-exports, constants, and the extension macro — under 60 lines
<!-- AC:END -->
