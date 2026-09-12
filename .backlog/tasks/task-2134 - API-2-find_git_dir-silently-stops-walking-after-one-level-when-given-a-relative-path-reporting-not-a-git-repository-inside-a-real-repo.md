---
id: TASK-2134
title: 'API-2: find_git_dir silently stops walking after one level when given a relative path, reporting ''not a git repository'' inside a real repo'
status: Done
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions/hook-common/src/git.rs
priority: medium
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:57` (`find_git_dir`)

**What**: the walk is

```rust
let mut dir = from.to_path_buf();
for _ in 0..FIND_GIT_DIR_MAX_DEPTH {
    if let Some(found) = probe_git_entry(&dir.join(".git")) { return Some(found); }
    if !dir.pop() { return None; }
}
```

`PathBuf::pop` returns `false` once the path has no parent. For a **relative**
input the walk therefore dies almost immediately: `Path::new("sub")` pops to
`""` (one probe of `./.git`, i.e. the process cwd), and the next `pop` on `""`
returns `false` — so the function returns `None` without ever looking above the
current working directory. `find_git_dir(Path::new("."))` behaves the same way:
one probe, then `None`. The documented contract is the opposite — the doc
promises it "walks up to the parent, up to [`FIND_GIT_DIR_MAX_DEPTH`] times"
and offers `FIND_GIT_DIR_MAX_DEPTH = 64` as the bound, with the note "Pass an
already-canonicalised input if the caller has a stricter containment
requirement" implying relative input is otherwise supported.

The precondition ("must be absolute") is real, load-bearing, and enforced
nowhere: the parameter is a plain `&Path`, and the failure mode is a silent
wrong answer, not an error. This is the "if documentation is required to
prevent misuse, the API is fragile" case — except the documentation does not
even state the precondition.

**Why it matters**: `find_git_dir` is public API of `ops-hook-common` and is
re-exported publicly again from `ops-git` (`extensions/git/src/config.rs:5`).
The in-tree callers happen to pass `std::env::current_dir()`
(`crates/cli/src/hook_shared.rs:126`, `extensions/git/src/provider.rs:42`), so
the bug is dormant today — but any caller that passes a relative path, or the
literal `"."`, gets `None` inside a perfectly good repository and the CLI
reports "not inside a git repository (no .git found)". Nothing in the type, the
signature, or the tests catches that: every existing test passes an absolute
tempdir path.

Fix options, in order of preference: canonicalize (or `absolute()`) the input
at the top of `find_git_dir` so the documented walk actually happens; or state
and enforce the precondition by taking a validated absolute-path newtype.
Either way the doc comment should stop describing a 64-level walk that a
relative input never gets.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 find_git_dir either normalizes a relative input to an absolute path before walking, or rejects/encodes the absolute-path precondition so a relative input cannot silently produce None
- [ ] #2 The doc comment matches the implemented behaviour for both absolute and relative inputs
- [ ] #3 A test walks up from a relative starting path (e.g. a subdirectory reached via a cwd guard) and finds the repo .git several levels above
- [ ] #4 Existing absolute-path tests, the FIND_GIT_DIR_MAX_DEPTH bound, and the SEC-14 pointer containment rules are unchanged
<!-- AC:END -->
