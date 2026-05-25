---
id: TASK-0999
title: >-
  ERR-7: extensions/about manifest_io and workspace use Display for tracing
  error fields, log injection vector
status: Done
assignee: []
created_date: '2026-05-04 22:01'
updated_date: '2026-05-05 00:28'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions/about/src/manifest_io.rs:47` (`error = %e`)
- `extensions/about/src/manifest_io.rs:63` (`error = %e`)
- `extensions/about/src/workspace.rs:83` (`error = %e`)
- `extensions/about/src/workspace.rs:172` (`error = %e`)

**What**: These `tracing::warn!` sites format `std::io::Error` via `%` (Display). Sister sites in the same file (workspace.rs:53, hook-common/git.rs:79 after TASK-0937) consistently use `?` (Debug) so embedded newlines / ANSI escapes / control bytes in error messages cannot forge log lines. The mixed posture means a hostile filename or symlink target whose error message includes `\n` or `\u{1b}[31m` survives unescaped at exactly the noise level (warn) operators read most often.

**Why it matters**: ERR-7 / log injection. The codebase has already done two sweeps (TASK-0818, TASK-0937) explicitly establishing the `?` policy for path/error fields in tracing events. These four sites are the remaining drift inside the audited extensions. A single regression here defeats the cross-file invariant.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All four cited tracing sites format error via the Debug formatter ().
- [ ] #2 A test (or doctest) pins that an io::Error containing a newline does not survive unescaped in the formatted field.
<!-- AC:END -->
