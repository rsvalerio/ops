---
id: TASK-1358
title: >-
  OWN-8: try_push receives name.clone() before the exclude/dedupe checks can
  reject it, contradicting its own 'allocates exactly once' docstring
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-12 21:28'
updated_date: '2026-05-17 09:16'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:159`

**What**: The caller iterates `config.commands` and passes `name.clone()` into `try_push`. The linear-scan dedupe + exclude check inside `try_push` then discards a non-trivial fraction of those clones. The rustdoc on `try_push` claims "each name allocates exactly once" — under exclude/dedup rejection it allocates *and immediately drops*.

**Why it matters**: OWN-8 unnecessary clone in a hot CLI-startup path (runs for every configured command at install-prompt time). Borrow the name as `&str` for the predicates and clone only on the successful-insert arm; update the doc comment to reflect the new invariant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Change try_push to take name: &str and clone only on the insert success branch
- [x] #2 Update the function-level doc to state 'surviving names allocate exactly once; rejected names never allocate' and add a regression test asserting no allocation on the exclude path (or document why a test is impractical)
<!-- AC:END -->
