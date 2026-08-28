---
id: TASK-1953
title: >-
  ERR-6: tracked_files collapses every git failure into None, silently widening
  the fixer from tracked files to the whole worktree
status: To Do
assignee:
  - TASK-2011
created_date: '2026-08-27 15:49'
updated_date: '2026-08-28 14:17'
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
- [ ] #1 tracked_files distinguishes 'not a git repository' and 'git not installed' from a genuine git failure, rather than returning one None for all of them
- [ ] #2 A genuine git failure under tracked_only is propagated to the caller instead of silently falling back to the full walk
- [ ] #3 When the walk fallback does fire under tracked_only, the run reports the downgrade on the writer so the user knows an unexpectedly wide file set may be rewritten
- [ ] #4 Tests cover all three outcomes: a real repo, a directory that is not a repo, and a git invocation that exits non-zero
<!-- AC:END -->
