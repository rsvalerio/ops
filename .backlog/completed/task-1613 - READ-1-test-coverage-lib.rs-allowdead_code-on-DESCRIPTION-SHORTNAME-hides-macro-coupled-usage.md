---
id: TASK-1613
title: >-
  READ-1: test-coverage lib.rs #[allow(dead_code)] on DESCRIPTION/SHORTNAME
  hides macro-coupled usage
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-22 06:49'
updated_date: '2026-05-22 10:16'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:32-36`

**What**: `DESCRIPTION` and `SHORTNAME` are passed to `ops_extension::impl_extension!`, but the constants also carry `#[allow(dead_code)]`. The compiler is telling the reader these names are unused under pure-Rust analysis, and the `#[allow]` papers over that without context.

**Why it matters**: READ-1 (explicit > implicit). A new reader cannot tell whether the constants are (a) consumed only via macro expansion and the allow is required, or (b) genuinely dead code from a prior refactor. If the macro inlines the literals at call site rather than referencing the consts, the allow is masking real dead code. A one-line `// referenced by impl_extension! macro expansion` comment (or removing the allow once verified) makes intent explicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either #[allow(dead_code)] is removed after confirming the macro keeps the constants live, or a comment explains why the lint is required
- [ ] #2 Build still passes without the allow if the constants are reachable through macro expansion
<!-- AC:END -->
