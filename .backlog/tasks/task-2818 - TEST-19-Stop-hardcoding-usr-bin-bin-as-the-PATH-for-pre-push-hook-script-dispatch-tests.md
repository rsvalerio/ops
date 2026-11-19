---
id: TASK-2818
title: 'TEST-19: Stop hardcoding /usr/bin:/bin as the PATH for pre-push hook-script dispatch tests'
status: To Do
assignee: []
created_date: '2026-11-18 18:47'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2862'
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-19:extensions/run-before-push/src/lib.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:443`, `:551`, `:600` (tests `hook_script_hands_ops_an_empty_stdin_and_forwards_refs_via_env`, `hook_script_dispatches_an_oversized_stream_without_e2big`, `hook_script_marks_truncation_when_byte_96001_is_a_newline`, via helper `run_hook_script`)

**What**: The dispatch tests build the child PATH as `format!("{}:/usr/bin:/bin", tempdir)` so the hook can find `head` and the fake `ops` can find `cat`. On non-FHS hosts (NixOS, some Nix/Guix CI images, minimal containers with coreutils elsewhere) `head` is not under `/usr/bin` or `/bin`, so the capture fails and these tests fail with the hook's "failed to read git's ref-update stream" diagnostic — a misleading failure that looks like a product regression.

**Why it matters**: Hardcoded system paths make the suite environment-dependent (TEST-19). The tempdir-first ordering already guarantees the fake `ops` shadows any real one, so the inherited PATH can be appended instead of a fixed list.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dispatch tests build PATH as the tempdir followed by the inherited PATH (std::env::var_os("PATH") / env::join_paths), not a literal /usr/bin:/bin
- [ ] #2 Tests that need no external tools (missing-ops guard, bypass) keep the tempdir-only PATH
- [ ] #3 The three dispatch tests still pass and the fake ops still shadows any real ops on PATH
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.29 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
