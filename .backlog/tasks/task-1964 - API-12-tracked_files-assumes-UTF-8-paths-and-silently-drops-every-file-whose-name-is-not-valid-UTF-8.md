---
id: TASK-1964
title: >-
  API-12: tracked_files assumes UTF-8 paths and silently drops every file whose
  name is not valid UTF-8
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:52'
updated_date: '2026-08-28 23:38'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions/text-fixers/src/discovery.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Low

**File**: `extensions/text-fixers/src/discovery.rs:94-97` (`tracked_files`)

Filed as API-12 because the ERR-6 slot for this file is taken by TASK-1953 (the git-failure fallback); this is a second, substantively different defect in the same function — the path-encoding assumption rather than the error-collapsing one.

**What**:

```
let Ok(rel) = std::str::from_utf8(chunk) else {
    continue;
};
out.push(root.join(rel));
```

The code correctly uses `git ls-files -z` (so filenames containing newlines or quotes are safe), and then throws that away by requiring the bytes to be valid UTF-8. On Unix a path is an arbitrary NUL-free byte sequence; `git ls-files -z` emits those bytes verbatim. A latin-1-named file committed on a non-UTF-8 locale machine, or any filename with a stray high byte, is silently dropped from the candidate set with no message.

The fix is one line and lossless: `std::os::unix::ffi::OsStrExt::from_bytes(chunk)` gives an `&OsStr` directly, no validation and no allocation. On Windows the UTF-8 assumption is fine, since git emits UTF-8 there.

Note the asymmetry: `walk` (discovery.rs:69-75) handles such paths fine — `ignore` yields `DirEntry` paths as `OsStr`. So the two discovery modes silently disagree on the file set for non-UTF-8 names, on top of the symlink disagreement in TASK-1947.

**Why it matters**: low impact — the failure mode is "a file is not fixed", not corruption — but it is a silent, mode-dependent scope difference in a tool whose exit code claims to mean "the tree is clean". API-12's point is that a UTF-8 path assumption should be a deliberate, stated contract rather than an accident of reaching for `str::from_utf8`; here it is neither stated nor needed.

**Suggested fix**: build the path from the raw bytes via `OsStr::from_bytes` under `#[cfg(unix)]`, keeping the `str::from_utf8` route for other targets. If dropping non-UTF-8 names really is intended, say so in the module doc and report the drop rather than swallowing it (see the ERR-1 finding on silent skips).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A tracked path whose bytes are not valid UTF-8 is included in the candidate set on unix rather than dropped
- [x] #2 The walk and tracked modes return the same file set for a fixture containing a non-UTF-8 filename, asserted by a test
- [x] #3 If any path is still dropped, the drop is reported rather than silently skipped
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. `str::from_utf8` is gone from the tracked path. `discovery::push_decoded` is `#[cfg(unix)]`-split: on Unix it builds the path from the raw bytes with `OsStr::from_bytes` — lossless, allocation-free, and infallible, so nothing can be dropped (AC#1). On other targets git emits UTF-8, so the `from_utf8` route stays there, and a chunk that still fails to decode is counted into `Discovery::undecodable_paths` rather than swallowed; `run_fixer` prints `N tracked path(s) skipped: filename is not valid UTF-8 on this platform` (AC#3).

The contract is stated in the discovery module doc under "# Path encoding".

AC#2: `discovery::tests::tracked_mode_keeps_a_non_utf8_filename_and_agrees_with_the_walk` creates a latin-1 `café.txt` (0xE9), stages it, and asserts the tracked set equals the walk set and `undecodable_paths == 0`.
<!-- SECTION:NOTES:END -->
