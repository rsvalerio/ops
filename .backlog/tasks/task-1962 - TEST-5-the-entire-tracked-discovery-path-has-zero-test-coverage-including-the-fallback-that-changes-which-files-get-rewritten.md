---
id: TASK-1962
title: >-
  TEST-5: the entire --tracked discovery path has zero test coverage, including
  the fallback that changes which files get rewritten
status: To Do
assignee:
  - TASK-2011
created_date: '2026-08-27 15:51'
updated_date: '2026-08-28 14:18'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/text-fixers/src/discovery.rs
  - extensions/text-fixers/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: High

**File**: `extensions/text-fixers/src/discovery.rs:38-45` (`discover`) and `:79-100` (`tracked_files`)

**What**: `discovery.rs` has four tests (lines 106-169) and every one of them calls `walk` directly. Nothing in the crate ever calls `discover` with `tracked_only = true`; `tracked_files` is never executed by any test. The same is true one level up — all three end-to-end tests in `lib.rs` construct `FixerOptions::new(root.to_path_buf(), false)` (lines 183, 209, 225), so the `tracked` flag is never exercised at any layer.

Untested, in a module whose output decides which of the user's files get rewritten in place:

- the `git ls-files -z` invocation and its `-C root` argument
- NUL-splitting of the output, and the `chunk.is_empty()` guard for the trailing separator
- `root.join(rel)` path reconstruction — including whether `git ls-files` paths are relative to `root` or to the repository root when `root` is a subdirectory of a repo, which is the difference between fixing the right files and fixing nothing
- the `!output.status.success()` branch
- the fallback-to-`walk` behaviour, which silently changes the operation's scope (see TASK-1953)
- the `from_utf8` branch that silently drops non-UTF-8 paths (see the companion API-12 finding)

`write_summary` (lib.rs:154-161) is also public and has no test.

**Why it matters**: `--tracked` is one of two modes of a destructive tool, and it is the mode a hook driver is most likely to use. It is also the mode that carries the symlink hazard (TASK-1947) — a hazard that exists precisely because nothing forces the two discovery modes to be compared. TEST-5 asks for at least one test per public API function; here a whole public branch is dark, and every other finding filed against this crate lands in code the test suite does not reach.

**Suggested fix**: add tests that build a real repository in a `tempfile::tempdir()` (`git init`, `git add`), then assert `discover(root, true)` returns exactly the tracked set and excludes an untracked file. Add a test asserting the fallback path from a non-repo directory. Add at least one end-to-end `run_trailing_whitespace` / `run_end_of_file_fixer` test with `tracked = true` so the two modes are compared on the same fixture tree. If shelling out to `git` in tests is unwanted, introduce a seam (a `fn(&Path) -> Option<Vec<PathBuf>>` injected through `FixerOptions`) so the parsing and the fallback logic can be tested without a subprocess — the parsing is where the path-reconstruction bugs live.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test initialises a real git repo in a tempdir and asserts discover with tracked_only true returns the tracked files and excludes an untracked one
- [ ] #2 A test asserts the behaviour when root is a subdirectory of a repository, pinning whether git ls-files paths are joined correctly
- [ ] #3 A test covers the non-repository fallback to walk
- [ ] #4 At least one end-to-end fixer test runs with FixerOptions tracked set to true
- [ ] #5 write_summary has a test asserting its exact output line
<!-- AC:END -->
