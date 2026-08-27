---
id: TASK-1912
title: >-
  READ-4: SKIP_ENV_VAR's doc comment still says 'when set to 1', but should_skip
  accepts 1/true/yes/on case-insensitively
status: Triage
assignee: []
created_date: '2026-08-27 15:40'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:37-38`

**What**:

    /// Environment variable that skips the run-before-push check when set to "1".
    pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_PUSH";

The implementation this const feeds is `ops_hook_common::should_skip` (`extensions/hook-common/src/lib.rs:88-99`), which accepts `"1"`, `"true"`, `"yes"` and `"on"`, case-insensitively. The hook-common doc comment is explicit that the narrower wording was the bug:

> documenting only `"1"` previously surprised users who set `SKIP_OPS_RUN_BEFORE_COMMIT=true`

That widening landed in `ops-hook-common`; the doc comment on this crate's const was never updated, so the stale `"1"`-only wording survives at the one place a reader looks it up. It is also the *only* documentation of the escape hatch anywhere — `grep -rn SKIP_OPS` finds no hit in `README.md` or `docs/`, and the README's command table (line 128) lists `ops run-before-push [install]` with no mention of how to bypass it.

The same stale wording is on `extensions/run-before-commit/src/lib.rs:46`.

**Why it matters**: READ-4 — the comment describes a contract narrower than the code's, on the one control an operator reaches for when a pre-push gate is blocking an emergency push. A reader who trusts it concludes `=true` will not work; a reader who tries `=true` and finds it does work has to go read another crate to learn what else is accepted. Neither reader can discover the option at all from the README.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The SKIP_ENV_VAR doc comment lists the values should_skip actually accepts (1, true, yes, on, case-insensitive) and states that anything else means do-not-skip
- [ ] #2 The doc links to ops_hook_common::should_skip as the source of truth so the two cannot drift again
- [ ] #3 SKIP_OPS_RUN_BEFORE_PUSH is documented in README.md alongside the run-before-push command entry
<!-- AC:END -->
