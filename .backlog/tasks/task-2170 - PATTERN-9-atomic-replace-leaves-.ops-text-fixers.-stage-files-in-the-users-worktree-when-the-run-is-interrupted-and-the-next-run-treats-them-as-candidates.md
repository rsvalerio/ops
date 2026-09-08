---
id: TASK-2170
title: >-
  PATTERN-9: atomic::replace leaves .ops-text-fixers.* stage files in the user's
  worktree when the run is interrupted, and the next run treats them as
  candidates
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-08 07:07'
updated_date: '2026-09-08 10:54'
labels:
  - code-review-rust
  - patterns
dependencies: []
modified_files:
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/discovery.rs
priority: low
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/atomic.rs:59-79`

**What**: `replace` stages into `tempfile::Builder::new().prefix(".ops-text-fixers.").tempfile_in(parent)`
and relies on `NamedTempFile`'s `Drop` to unlink the stage on any error path.
`Drop` does not run when the process is killed — SIGINT/SIGTERM with no
handler, SIGKILL, or a hard CI cancel — which is precisely the scenario the
module header names as the motivation for the whole design: "Ctrl-C on a
pre-commit hook … in that window".

So the interruption that used to truncate a source file now instead leaves a
`.ops-text-fixers.XXXXXX` file **next to it**, one per file that was mid-write,
scattered wherever in the tree the run had reached. Two consequences:

1. They are not cleaned up by anything. Nothing in the crate looks for stale
   stages on a later run.
2. The next fixer run picks them up as candidates. `discovery.rs` builds the
   walker with `hidden(false)` ("keeps dotfiles … in scope"), the prefix is
   not in `SKIP_DIRS`, and there is no `.gitignore` entry for it — so the
   leftovers are walked, read, and (being copies of already-fixed content)
   counted in `files_scanned`. They also show up in `git status` as untracked
   files, and under `--tracked` a user who ran `git add -A` after an
   interrupted run would commit them.

**Why it matters**: low individually, but it is the residue of the crate's
central safety trade and the module doc does not acknowledge it — the "Trade
this makes" section lists hard links and open file descriptors, not stale
stages. A pre-commit hook is the most-interrupted program a developer runs.

Options, roughly in order of cost: add `.ops-text-fixers.*` to the deny-list
or the walker filter so the leftovers cannot become candidates; sweep stale
stages older than some age at the start of a run; or install a signal handler
that unlinks the in-flight stage. At minimum, document the residue alongside
the other two accepted trades.

<!-- scan confidence: verified — atomic.rs and discovery.rs walker config read in full -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A leftover .ops-text-fixers.* stage file is never a candidate for a subsequent fixer run
- [ ] #2 A test creates a stale stage file in a tree and asserts discovery does not return it
- [ ] #3 The atomic.rs 'The trade this makes' section names the stale-stage residue alongside hard links and open file descriptors, or the residue is eliminated
<!-- AC:END -->
