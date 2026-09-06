---
id: TASK-1668
title: 'ARCH-11 follow-up: enable the three deferred documentation lints'
status: Done
assignee: []
created_date: '2026-08-16 09:42'
updated_date: '2026-08-16 10:52'
labels:
  - rust-code-review
  - arch
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-0137 turned on `clippy::pedantic` workspace-wide but allowed three documentation lints in `[workspace.lints.clippy]`, because enforcing them meant a ~730-site documentation sweep that would have buried the rest of the policy change. The allows carry a comment saying they are 'tracked separately' — this is that tracking.

Measured against main on 2026-08-16 with `--all-features --all-targets`, deduplicated by (lint, file, line):

| Lint | Sites |
|---|---|
| `clippy::doc_markdown` | 409 |
| `clippy::must_use_candidate` | 210 |
| `clippy::missing_errors_doc` | 113 |

Re-measure one at a time with:

    cargo clippy --workspace --all-features --all-targets -- -W clippy::doc_markdown

Each lint is independent, so this splits into three separate pieces of work and does not need to land as one change. `doc_markdown` is the largest but also the most mechanical (backticks around identifiers in doc comments). `missing_errors_doc` is the smallest and the most valuable — it forces every public fallible function to document its failure modes.

Policy is documented in `docs/clippy.md`; update the 'Deferred' table there as each lint is enabled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 clippy::missing_errors_doc is enforced and its allow removed from [workspace.lints.clippy]
- [x] #2 clippy::must_use_candidate is enforced and its allow removed
- [x] #3 clippy::doc_markdown is enforced and its allow removed
- [x] #4 docs/clippy.md no longer lists the enabled lints as deferred
- [x] #5 ops verify and ops qa pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done. All three lints are enforced; `[workspace.lints.clippy]` no longer has a
deferred group at all.

| Lint | Sites | How |
|---|---|---|
| `doc_markdown` | 409 | `cargo clippy --fix` cleared 407; 2 needed hand-fixing |
| `must_use_candidate` | 210 | `cargo clippy --fix` cleared all 210 |
| `missing_errors_doc` | 113 | hand-written, 54 files |

**The two mechanical lints were genuinely mechanical.** I audited both `--fix`
diffs line by line given the damage `--fix` did under TASK-0137: the
`must_use_candidate` pass added 210 `#[must_use]` attributes and changed
nothing else, and the `doc_markdown` pass touched only `///` and `//!` lines.
Verified by diffing added lines and confirming the only non-doc, non-attribute
changes were the 76 already belonging to TASK-1666/1669.

The 2 `doc_markdown` stragglers were a real defect rather than a formatting
nit: `crates/cli/src/registry/tests.rs` had a backtick opened on one line and
closed on the next (`\`compiled.len()` / `>= filtered.len()\``), which rustdoc
renders as unbalanced. Joined onto one line.

**`missing_errors_doc` was the real work** — 113 functions across 54 files, each
needing its actual failure modes read out of the body rather than a boilerplate
sentence. Where a distinction exists it is stated: the DuckDB query helpers
note that a *missing table is not an error* (they yield `0` / an empty vec),
`read_config_file` notes a missing file is `Ok(None)`, and `discover` notes a
failing `git ls-files` falls back to the full walk rather than erroring.
Functions returning typed errors name the variants (`SqlError::EmptyPath`,
`DbError::NonUtf8Path`, `HasStagedFilesError`) so the docs stay checkable.

Docs: `docs/clippy.md` no longer lists a deferred group, and the site-local
census gained the third `unnecessary_debug_formatting` entry added by TASK-1669.

Gates: `ops verify` 7/7, `ops qa` 3/3.
<!-- SECTION:NOTES:END -->
