---
id: TASK-0720
title: >-
  TEST-11: extension_constants tests assert string equality with their own
  definition
status: To Do
assignee:
  - TASK-0740
created_date: '2026-04-30 05:31'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:358` and `extensions/run-before-push/src/lib.rs:91`

**What**: Both crates have an `extension_constants` test that asserts `NAME == "run-before-commit"` (or `-push`), `SHORTNAME == NAME`, and `!DESCRIPTION.is_empty()`. The literal that the test compares against is the same `pub const NAME: &str = "run-before-commit"` declared a few lines above; flipping that const flips the test in lockstep, so the assertion has no separate verification value.

**Why it matters**: TEST-11 (assert specific *values*, not surface tautologies). The intent appears to be "pin the public identifier so a rename is loud", but the current shape only catches `let NAME = "" ;`. A meaningful pin would: (a) compare against an external source of truth (the macro-derived shortname registered in the extension registry, or the hook script's `exec ops <NAME>` token) or (b) assert a structural property (NAME matches `^[a-z][a-z-]+$`, SHORTNAME matches a documented pattern). Otherwise the test is a copy of the const.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the self-referential equality assertion with a structural check (e.g. assert HOOK_SCRIPT contains "ops {NAME}") or compare against an external registry value
- [ ] #2 Apply the same fix to both run-before-commit and run-before-push so they remain in lockstep
- [ ] #3 Keep coverage of the !DESCRIPTION.is_empty() invariant if you want a startup-string check
<!-- AC:END -->
