---
id: TASK-1886
title: >-
  ERR-7: install.rs and git_state.rs log attacker-influenced paths and env
  values with Display, contradicting the crate's own Debug-escaping policy
status: Done
assignee:
  - TASK-2008
created_date: '2026-08-27 15:33'
updated_date: '2026-08-28 23:00'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/hook-common/src/install.rs
  - extensions/hook-common/src/git_state.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:241-254`, `extensions/hook-common/src/git_state.rs:54-71`, `extensions/hook-common/src/git_state.rs:89-93`

**What**: `git.rs` establishes an explicit crate policy (ERR-7 / TASK-0937): every path or error that reaches a `tracing` field is formatted with `?` (Debug) so embedded newlines and ANSI escapes cannot forge log lines. It is documented four times in `read_gitdir_pointer` and pinned by a regression test, `git_pointer_path_debug_escapes_control_characters` (`git.rs:222`), which asserts the rendered value contains no raw `\n` and no `\u{1b}`.

Three sites in the same crate use `%` (Display) instead, on values that are exactly as externally influenced as the ones `git.rs` guards:

- `install.rs:242-243` and `install.rs:250-251` — `parent = %parent.display(), error = %e` inside `sync_parent_dir`. `parent` is `<git_dir>/hooks`, derived from a repository path the user did not necessarily create (a cloned or unpacked tree can contain a directory name with a newline or an ESC sequence).
- `git_state.rs:56-58` — `value = %raw` in `git_timeout_from_env`, where `raw` is the **raw environment variable value**, logged on the unparseable branch. This is the most directly attacker-supplied string in the crate: it is logged precisely *because* it is garbage, and the warn fires whenever the value fails to parse.
- `git_state.rs:90-91` — `program = %program` in `read_stderr_bounded` (the program name is caller-supplied and reaches the same operator-facing stream).

**Why it matters**: a hook runs on every commit and its warnings land in the developer's terminal. `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS=$'10s\nWARN forged log line'` reproduces the forgery the ERR-7 sweep was opened to close, and an ESC-bearing value can rewrite what the terminal shows around it. The defect is not that Display is wrong in general — it is that this crate declared a policy, wrote a test for it, and then left three sites on the other side of it, so the guarantee readers infer from `git.rs` does not hold crate-wide. Switching the four fields to `?` is a mechanical change; extending the existing escape test to cover a `git_state` value would keep the policy honest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 sync_parent_dir formats its parent path and io::Error tracing fields with ? (Debug) rather than %
- [x] #2 git_timeout_from_env formats the raw env value with ? so control characters and ANSI escapes are escaped in the warn line
- [x] #3 read_stderr_bounded formats program with ?
- [x] #4 A test asserts a control-character-bearing env value is escaped in the git_timeout_from_env warn output, mirroring git_pointer_path_debug_escapes_control_characters
<!-- AC:END -->
