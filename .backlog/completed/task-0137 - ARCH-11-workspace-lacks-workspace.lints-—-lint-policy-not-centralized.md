---
id: TASK-0137
title: 'ARCH-11: workspace lacks [workspace.lints] — lint policy not centralized'
status: Done
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-08-15 20:45'
labels:
  - rust-code-review
  - arch
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `Cargo.toml` (workspace root)

**What**: The workspace defines [workspace.dependencies] for version alignment but not [workspace.lints]. Individual crates can (and do) drift in clippy/rustc lint level.

**Why it matters**: Inconsistent lint enforcement lets warnings slip in for some crates while others are strict; ARCH-11 calls for unified lint policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add [workspace.lints] with agreed clippy/rustc categories and levels (at minimum clippy::pedantic warn, clippy::unwrap_used in non-test)
- [x] #2 Each crate/extension Cargo.toml sets [lints] workspace = true
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done.

`[workspace.lints]` now carries the policy and all 28 members already had
`[lints] workspace = true` (AC #2 was met before this change).

**rust lints**: `elided_lifetimes_in_paths`, `unsafe_op_in_unsafe_fn`,
`unused_lifetimes`. Deliberately NOT `rust_2018_idioms` — it implies
`unused_extern_crates`, and `cargo clippy --fix` used it to delete the
`extern crate ops_git` / `extern crate ops_text_fixers` lines in
crates/cli/src/main.rs that exist solely to stop the linker dropping
linkme-only extensions. Caught and reverted; `ops extension list` confirms
both still register.

**clippy**: `all` + `pedantic` at warn, `unwrap_used` at warn. Production code
had exactly 1 unwrap (extensions-rust/deps/src/format.rs, now an `if let`);
the other ~1650 are in tests, which opt out via
`#![cfg_attr(test, allow(...))]` in each of the 28 crate roots (also covers
the three cast lints, which are pure noise in test fixtures).

~1100 pedantic warnings were resolved: `cargo clippy --fix` handled the
mechanical bulk, the remaining 156 were fixed by hand or given site-local
allows with reasons.

Workspace-level allows, each with a justification comment in Cargo.toml:
- doc_markdown / missing_errors_doc / must_use_candidate — ~500 doc comments,
  deferred documentation sweep
- duration_suboptimal_units — its fix (`Duration::from_mins`) needs Rust
  ~1.92 and silently raised the declared `rust-version = "1.80"`; `--fix`
  had applied it to 9 timeout constants, all reverted
- items_after_statements / struct_excessive_bools / similar_names — style
  preferences this codebase deliberately does not share
- needless_pass_by_value — 19 sites needing signature + call-site changes,
  some constrained by the runner's `'static` spawn bounds; filed as TASK-1666

Gates: `ops verify` 7/7, `ops qa` 3/3 (including --ignored).

Note for a future task: `rust-version = "1.80"` is already inaccurate — the
tree uses `unsafe extern "C"` blocks (1.82+). Nothing in CI or clippy.toml
enforces MSRV, so the drift is currently invisible.
<!-- SECTION:NOTES:END -->

## Triage Notes

<!-- SECTION:TRIAGE:BEGIN -->
Reset from `In Progress` to `To Do` in the 2026-08-15 sweep.

Verified against the tree: `Cargo.toml` still has no `[workspace.lints]`
table, so neither AC is met and no work has landed. The `In Progress` marker
dated 2026-04-23 was stale, not a work-in-progress.
<!-- SECTION:TRIAGE:END -->
