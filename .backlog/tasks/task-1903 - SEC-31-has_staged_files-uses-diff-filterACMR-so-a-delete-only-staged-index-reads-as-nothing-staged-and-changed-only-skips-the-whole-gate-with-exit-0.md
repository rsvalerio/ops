---
id: TASK-1903
title: >-
  SEC-31: has_staged_files uses --diff-filter=ACMR, so a delete-only staged
  index reads as 'nothing staged' and --changed-only skips the whole gate with
  exit 0
status: To Do
assignee:
  - TASK-2009
created_date: '2026-08-27 15:38'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:73-84` (`has_staged_files`)

**What**: The doc comment on this crate's public preflight states an unqualified contract:

```
/// Returns `true` if there are any staged files in the git index.
pub fn has_staged_files() -> anyhow::Result<bool> {
```

The probe it delegates to runs (cross-crate cause, `extensions/hook-common/src/git_state.rs:148-150`):

```
.args(["diff", "--cached", "--quiet", "--diff-filter=ACMR"])
```

`--diff-filter=ACMR` selects only Added / Copied / Modified / Renamed. It excludes:

- **D** — staged deletions (`git rm foo.rs`, `git add -A` after a delete)
- **T** — staged type changes (file -> symlink, regular -> submodule gitlink)
- **U** — unmerged / conflicted paths

For a commit whose index contains only those, `git diff --cached --quiet --diff-filter=ACMR` exits 0, the probe returns `Ok(false)`, and `crates/cli/src/subcommands.rs:221-227` prints `[run-before-commit] no staged files — skipping` and returns `ExitCode::SUCCESS`. The user's entire configured `[commands.run-before-commit]` chain (fmt, clippy, tests) never runs, and git sees a clean exit.

Deleting a file is one of the *most* likely ways to break a build (a removed module, a removed fixture, a removed `mod` target), and a conflicted index is precisely the state a pre-commit gate exists to catch. The predicate answers a narrower question than its name, its doc, and its call site all assume.

**Why it matters**: SEC-31 — a gate must fail closed. This one fails open on an entire class of commits, silently and with exit 0, so neither the developer nor CI has any signal that the checks were skipped. It is the same shape as TASK-1754 (`ops sec` with every scan skipped exits 0 reporting a clean scan) and TASK-1818 (`validate_commands` has no production caller). The ACMR filter also has no comment anywhere explaining why deletions were excluded, so a reader cannot tell whether this is intentional policy or a copy-paste from a `--name-only` era predecessor (it survives from the original implementation flagged in TASK-0143).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 has_staged_files returns true when the index contains only staged deletions (D)
- [ ] #2 has_staged_files returns true when the index contains only unmerged paths (U) or type changes (T), or the doc and the CLI skip message state explicitly which change kinds are ignored and why
- [ ] #3 A test stages only a deletion (git rm) in a temp repo and asserts the predicate reports staged work
- [ ] #4 The choice of diff-filter carries a comment stating the rationale
<!-- AC:END -->
