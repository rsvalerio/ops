---
id: TASK-1974
title: >-
  CL-3: TOKEI_DEFAULT_EXCLUDED entries are gitignore globs that match files and
  any directory depth, not the top-level build dirs the doc describes
status: Done
assignee:
  - TASK-2012
created_date: '2026-08-27 15:55'
updated_date: '2026-08-28 15:59'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:105-126`

**What**: The constant is documented as a list of build-artifact and VCS **directories**:

    /// Directories excluded from cargo-style projects' tokei scan.
    pub(crate) const TOKEI_DEFAULT_EXCLUDED: &[&str] = &[
        "target", ".git", "node_modules", ".venv", "venv", "dist", "build",
    ];

It is not treated that way. `Languages::get_statistics` forwards the slice to tokei's walker, which does `overrides.add(&format!("!{}", ignored))` for each entry (tokei-14.0.0/src/utils/fs.rs). In the `ignore` crate an override glob has gitignore semantics with `!` inverted, and a gitignore pattern containing no slash matches **by basename, at any depth, files as well as directories**. So the real effect of this list is:

- Any directory named `build`, `dist`, `venv` or `target` **anywhere** in the tree is dropped, including legitimate source directories -- a Python package `pkg/build/`, a JS source folder `src/dist/`, a Go package named `venv`, a Java module directory `target`. Their code silently does not exist in the reported statistics.
- Plain **files** with those names are dropped too -- a shell script named `build`, a Makefile-adjacent `dist` -- because the pattern is not anchored to a directory.
- Nothing is anchored to the workspace root, which is what the doc comment implies and what a reader will assume when adding the next entry to the list.

Compounding it, the list is largely redundant: tokei's walker enables `git_ignore`, `git_global`, `git_exclude` and `hidden` by default, so in any git repository `target/`, `node_modules/`, `dist/`, `build/` and `.git` are already excluded by the project's own `.gitignore`. What the constant adds over that default is mostly the over-matching, not the exclusion. It is also hardcoded with no way for a project to override or extend it from `.ops.toml`, so a project that legitimately keeps source under one of these names has no escape hatch.

Test coverage does not catch any of this: `collect_tokei_excludes_target_and_git` (`extensions/tokei/src/tests.rs:121-150`) creates the excluded directories only at the top level, and `tokei_default_excluded_contains_expected_dirs` (152-159) asserts membership for 4 of the 7 entries. No test places a `build/` or `dist/` directory nested under `src/`, which is the case that silently loses data.

**Why it matters**: CL-3 -- the doc states an assumption ("directories", implicitly at the project root) that the implementation does not hold, so anyone extending the list will pick a name expecting root-anchored directory semantics and get an any-depth basename match over files too. The failure is silent: files vanish from the LOC report with no warning, and the consumer of `tokei_files` / `tokei_languages` has no way to notice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Exclusion patterns are anchored so they match directories at the workspace root only, or the doc comment is corrected to state the real any-depth basename-over-files semantics
- [x] #2 A test places a directory named build or dist nested under src/ and asserts the intended behaviour for it
- [x] #3 The redundancy with tokei's default gitignore handling is either removed or documented as deliberate
- [x] #4 Whether the exclusion list can be overridden per project is decided and recorded next to the constant
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2012 (branch code-review/TASK-2012).

- AC #1: the list is no longer handed to tokei's override builder, so it is no
  longer gitignore-glob semantics. The walk is now the crate's own
  `ignore::WalkBuilder`, and `is_pruned_dir` prunes only a *directory* that is a
  *direct child of the scan root* (`entry.depth() == 1`). Root-anchored
  directory matching is now what the code does and what the doc says.
- AC #2: `collect_tokei_counts_build_dir_nested_under_src` puts real source in
  `src/build/gen.rs` and asserts it is counted while root-level `build/` is
  pruned. `collect_tokei_counts_file_named_like_an_excluded_dir` pins that a
  plain file named `build` is source, not an exclusion.
- AC #3: the redundancy with tokei's gitignore defaults is documented as
  deliberate on the constant -- the list is what keeps counts sane on an
  unversioned checkout with no ignore file, the same call `rust-loc` makes.
- AC #4: recorded next to the constant -- deliberately not project-overridable,
  because a project that keeps source under one of these names can negate it in
  its own `.gitignore`, which the walker reads, and a second tokei-only
  exclusion dialect would reach no further.

Follow-up filed: TASK-2016 (rust-loc's `EXCLUDED_DIRS` still prunes at any
depth while citing this constant as the policy it mirrors).
<!-- SECTION:NOTES:END -->
