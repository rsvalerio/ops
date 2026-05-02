---
id: TASK-0900
title: >-
  READ-5: resolve_spec_cwd strips non-UTF-8 bytes from spec.cwd via
  to_string_lossy
status: Done
assignee: []
created_date: '2026-05-02 10:08'
updated_date: '2026-05-02 14:47'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:178`

**What**: resolve_spec_cwd calls `p.to_string_lossy()` on the spec cwd before variable expansion, replacing non-UTF-8 bytes with U+FFFD. The mangled path then flows through `try_expand` and `current_dir`, so a cwd containing non-UTF-8 bytes spawns the child in a wrong-but-superficially-similar directory rather than failing loudly.

**Why it matters**: A misspelled byte sequence quietly redirects build output to the wrong location instead of failing loudly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_spec_cwd accepts non-UTF-8 OsStr cwds without expansion, OR returns ExpandError when expansion is requested but the input is not UTF-8
- [x] #2 Add a unix-only test using OsString::from_vec(vec![0xff, ...]) that asserts the failure mode is loud, not lossy
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
resolve_spec_cwd now uses Path::to_str() and returns std::io::Error InvalidInput on non-UTF-8 cwd values, instead of lossy-replacing bytes with U+FFFD and silently chdir-ing the child into a wrong-but-similar path. Added Unix-only test resolve_spec_cwd_rejects_non_utf8_cwd_loudly using OsString::from_vec(vec![b\\"sub\\", 0xff]) that asserts the error kind and message.
<!-- SECTION:NOTES:END -->
