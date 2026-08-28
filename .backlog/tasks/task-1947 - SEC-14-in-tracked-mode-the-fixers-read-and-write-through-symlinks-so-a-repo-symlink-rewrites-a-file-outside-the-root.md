---
id: TASK-1947
title: >-
  SEC-14: in --tracked mode the fixers read and write through symlinks, so a
  repo symlink rewrites a file outside the root
status: To Do
assignee:
  - TASK-2011
created_date: '2026-08-27 15:48'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/text-fixers/src/discovery.rs
  - extensions/text-fixers/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: High

**File**: `extensions/text-fixers/src/discovery.rs:79-100` (`tracked_files`) and `extensions/text-fixers/src/lib.rs:128-148` (`run_fixer`)

**What**: the two discovery modes disagree about symlinks, and only one of them is safe.

- `walk` (discovery.rs:69-75) is careful: `follow_links` is off, so a symlink surfaces as a symlink entry, `entry.file_type().is_some_and(|t| t.is_file())` is false, and it is dropped. The comment on line 70-71 says exactly this.
- `tracked_files` has no such check. `git ls-files -z` lists symlinks (index mode 120000) as ordinary path entries, and every chunk is turned into `root.join(rel)` and pushed with no `symlink_metadata` / `is_file` test.

`run_fixer` then does `std::fs::read(&path)` (follows the link, reads the *target*) and `std::fs::write(&path, &fixed)` (follows the link, truncates and rewrites the *target*). Nothing in the crate ever calls `symlink_metadata`, `canonicalize`, or checks containment under `opts.root`.

So a repo that tracks `docs/config -> ../../../etc/myapp/app.conf`, or the very common `link -> ../../shared/file.txt` pointing outside the checkout, has that out-of-root file rewritten by `ops trailing-whitespace --tracked` / `ops end-of-file-fixer --tracked`. The path is attacker-influenced in the ordinary supply-chain sense: cloning an untrusted repo and running the project's own pre-commit hook is enough. Combined with TASK-1943 (non-atomic write) the out-of-root file is also truncated for the duration of the write.

A related sub-case: `git ls-files` can list a symlink to a device or FIFO, in which case `fs::read` blocks or reads unbounded — the same shape as TASK-1811 filed against `extensions/config-checkers/src/lib.rs`, which reaches this hazard by calling `ops_text_fixers::discovery::discover` (config-checkers/src/lib.rs:223). Fixing it here fixes both consumers; the config-checkers task covers only that crate's own read.

**Why it matters**: writes outside the intended root, driven by repository-controlled data, on a path that runs automatically from a git hook. The asymmetry between the two discovery modes also means the safety property is accidental rather than designed — nothing tells a future editor that `walk`'s `is_file()` was load-bearing.

**Suggested fix**: make the symlink policy one explicit decision applied in `discover`, not two accidents. Filter both modes through a shared predicate that uses `symlink_metadata` (never `metadata`) and keeps only regular files, and document that symlinks are out of scope for a fixer that rewrites in place. If following links is ever wanted, it must be gated on the canonicalized target still living under `opts.root`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tracked_files filters entries through symlink_metadata and keeps only regular files, so a tracked symlink is never returned
- [ ] #2 The symlink policy is applied in one shared place used by both the walk and tracked modes, and stated in the discovery module docs
- [ ] #3 A test creates a tracked symlink pointing at a file outside the temp root, runs both fixers in tracked mode, and asserts the outside file is byte-identical afterwards
- [ ] #4 A test asserts walk mode and tracked mode return the same set of paths for a fixture containing a symlink
<!-- AC:END -->
