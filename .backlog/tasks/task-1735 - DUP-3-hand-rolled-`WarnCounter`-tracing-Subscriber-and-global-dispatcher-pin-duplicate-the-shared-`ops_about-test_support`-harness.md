---
id: TASK-1735
title: >-
  DUP-3: hand-rolled `WarnCounter` tracing Subscriber and global-dispatcher pin
  duplicate the shared `ops_about::test_support` harness
status: To Do
assignee:
  - TASK-1989
created_date: '2026-08-27 11:12'
updated_date: '2026-08-28 14:11'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-go/about/src/modules.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:357-397`

**What**: The `modules` test module hand-rolls a full `tracing::Subscriber`
implementation (`WarnCounter`, 18 lines of trait methods, seven of which
are empty stubs) plus a `pin_global_dispatcher` helper that installs a
**process-global** default subscriber via
`tracing::subscriber::set_global_default` and calls
`tracing::callsite::rebuild_interest_cache()` to work around `tracing`'s
process-wide `Interest` cache. Two tests use it
(`collect_units_dotdot_prefixed_dir_is_in_tree`,
`collect_units_absolute_use_directive_is_marked_out_of_tree`).

`extensions/about/src/test_support.rs` exists precisely to own this kind of
harness — its module doc records DUP-3 / TASK-1157, where three
crate-local `BufWriter(Arc<Mutex<Vec<u8>>>)` copies were consolidated into
`TracingBuf` because "style drift between copies led to inconsistent log
capture". This crate already carries
`ops-about = { workspace = true, features = ["test-support"] }` in its
dev-dependencies (Cargo.toml:20) and already calls
`ops_about::test_support::assert_debug_escapes_control_chars`
(modules.rs:349) — it just never used the tracing half.

The "install a global sink subscriber and swallow the error" pattern now
has five copies in the workspace (cross-crate cause, listed for context —
the fix belongs in this crate's file plus the shared module):

- `extensions-go/about/src/modules.rs:392` (this finding)
- `extensions/git/src/config.rs:772`
- `extensions-rust/cargo-update/src/tests.rs:62`
- `crates/core/src/test_utils.rs:596`
- `crates/cli/src/test_utils.rs:129`

`TracingBuf` captures rendered text, so a level-counting variant does not
exist yet; the fix is to add one (e.g. `LevelCounter` / `warn_count()`)
alongside `TracingBuf` rather than to keep a bespoke `Subscriber` impl
inside a production module file.

**Why it matters**: the local copy is the subtlest of the five — the
`pin_global_dispatcher` comment documents a real, silent-flake hazard
(`Interest::never()` cached by a parallel test thread) that every future
copy of this pattern will rediscover the hard way. Centralising it means
the workaround is written once and inherited, which is exactly the argument
TASK-1157 already made for `TracingBuf`. Secondary: 40 lines of test
scaffolding sit in the middle of a 525-line source file, between the unit
assertions they serve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A level-counting tracing test harness lives in `extensions/about/src/test_support.rs` behind the `test-support` feature, alongside `TracingBuf`, carrying the `Interest`-cache/global-dispatcher workaround and its rationale
- [ ] #2 `extensions-go/about/src/modules.rs` deletes the local `WarnCounter` and `pin_global_dispatcher` and uses the shared harness
- [ ] #3 `collect_units_dotdot_prefixed_dir_is_in_tree` and `collect_units_absolute_use_directive_is_marked_out_of_tree` still assert the same warn counts (0 and 1) and remain non-flaky under `cargo test` (shared-process threads) as well as nextest
<!-- AC:END -->
