---
id: TASK-1953
title: >-
  ERR-6: tracked_files collapses every git failure into None, silently widening
  the fixer from tracked files to the whole worktree
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:49'
updated_date: '2026-08-28 23:37'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions/text-fixers/src/discovery.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: High

**File**: `extensions/text-fixers/src/discovery.rs:38-45` (`discover`) and `:79-100` (`tracked_files`)

**What**: `tracked_files` returns `Option<Vec<PathBuf>>` and returns `None` for four materially different outcomes:

1. git is not installed (`Command::new("git").output().ok()?`)
2. `root` is not inside a git repository (non-zero exit)
3. `git ls-files` failed for any other reason — corrupt index, `dubious ownership` safe.directory refusal, a lock held by a concurrent git process, ENOMEM
4. a genuinely empty repo (this one returns `Some(vec![])`, so it is at least distinguishable — but only by accident)

`discover` then treats `None` as "just walk everything":

```
if tracked_only {
    if let Some(paths) = tracked_files(root) { return Ok(paths); }
}
walk(root)
```

The caller asked for `--tracked`, meaning "only files git knows about". What it silently gets instead is every non-ignored file in the tree, **including untracked ones** — and this crate does not merely read those files, it rewrites them in place. Nothing is printed, nothing is logged, the `FixerReport` carries no indication that the mode was downgraded, and the doc comment at line 34-37 presents the fallback as intended behaviour ("A failing `git ls-files` under `tracked_only` is not an error").

Case 3 is the one that bites: `dubious ownership` is the default state for a repo checked out under a different uid (containers, CI, shared build agents, `sudo`), and it makes `git ls-files` exit non-zero on a directory that *is* a repo. In that environment `ops trailing-whitespace --tracked` quietly rewrites the user's untracked scratch files.

**Why it matters**: an error-handling shortcut changes the *scope of a destructive operation* without telling anyone. ERR-6 ("`Result<Option<T>>` over sentinel values") is exactly this: the `None` sentinel erases the distinction between "not a repo, fall back" (a legitimate, intended fallback) and "git is broken here" (an error the caller must see). It is also the reason the SEC-14 symlink hazard and the SEC-25 truncation hazard reach a wider file set than a reviewer reading `--tracked` would expect.

**Suggested fix**: return `std::io::Result<Option<Vec<PathBuf>>>` (or a small enum: `Tracked(Vec<_>)` / `NotARepository` / `Err`). Fall back to `walk` only for the "not a repository" and "git not installed" cases; propagate everything else. Whichever way it goes, when the fallback fires under `tracked_only` the run must say so on the writer, since it changes which of the user's files get rewritten.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tracked_files distinguishes 'not a git repository' and 'git not installed' from a genuine git failure, rather than returning one None for all of them
- [x] #2 A genuine git failure under tracked_only is propagated to the caller instead of silently falling back to the full walk
- [x] #3 When the walk fallback does fire under tracked_only, the run reports the downgrade on the writer so the user knows an unexpectedly wide file set may be rewritten
- [x] #4 Tests cover all three outcomes: a real repo, a directory that is not a repo, and a git invocation that exits non-zero
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. `tracked_files` now returns `io::Result<Tracked>` with three outcomes: `Files { files, undecodable }`, `NotARepository` (git ran and said so — detected from stderr), and `GitUnavailable` (spawn failed with `NotFound`). Every other non-zero exit becomes an `io::Error` naming the root, the exit status and git's stderr — so `dubious ownership`, a corrupt index, a held lock or ENOMEM propagate instead of silently widening a destructive tool from the index to the whole worktree.

`discover` returns a `Discovery { files, fallback, undecodable_paths, walk_errors }`. AC#3: when the fallback fires, `run_fixer` writes `--tracked unavailable (<reason>); falling back to a full walk of <root> — untracked files are candidates too` before touching anything. `ops-config-checkers`, the other consumer of `discover`, prints the equivalent line.

AC#4 tests: `tracked_mode_returns_tracked_files_and_excludes_untracked` (real repo), `tracked_mode_falls_back_and_reports_when_root_is_not_a_repository` (non-repo, asserts `Fallback::NotARepository` and that the walk set includes the untracked file), `a_genuine_git_failure_is_an_error_not_a_silent_fallback` (index corrupted after `git add`; asserts `Err`). Plus `tests::a_tracked_run_outside_a_repository_announces_the_downgrade` end to end. The git-not-installed arm has no fixture: forcing it needs a process-wide `PATH` mutation, which is unsafe under a parallel test runner; it is one `ErrorKind::NotFound` match.
<!-- SECTION:NOTES:END -->
