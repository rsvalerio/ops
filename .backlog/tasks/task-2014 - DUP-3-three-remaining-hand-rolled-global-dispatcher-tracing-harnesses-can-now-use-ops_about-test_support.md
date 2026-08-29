---
id: TASK-2014
title: >-
  DUP-3: three remaining hand-rolled global-dispatcher tracing harnesses can now
  use ops_about::test_support
status: Done
assignee:
  - TASK-2045
created_date: '2026-08-28 15:33'
updated_date: '2026-08-29 13:52'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - crates/core/src/test_utils.rs
  - crates/cli/src/test_utils.rs
  - extensions/git/src/config.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/test_utils.rs:596`, `crates/cli/src/test_utils.rs:129`, `extensions/git/src/config.rs:772`

**What**: TASK-1735 consolidated the `extensions-go/about` copy of the
"install a global sink subscriber, swallow the error, `rebuild_interest_cache()`"
idiom into `ops_about::test_support` (new `count_warnings` / `WarnCounter`,
alongside the existing `TracingBuf`). Three of the five sites TASK-1735
enumerated still carry their own copy of the pin plus its rationale comment:

- `crates/core/src/test_utils.rs:596`
- `crates/cli/src/test_utils.rs:129`
- `extensions/git/src/config.rs:772`

(The fourth, `extensions-rust/cargo-update/src/tests.rs:62`, is already filed
as TASK-1794.)

**Why it matters**: the `Interest`-cache workaround these copies encode guards a
*silent flake*, not a failure — a parallel test thread that first-hits a warn
callsite with no dispatcher registered caches `Interest::never()` and the warn
assertion fails at random. Every copy that drifts rediscovers that the hard way,
which is exactly the argument TASK-1157 made for centralising `TracingBuf`.
Now that the shared harness exists, each remaining copy is pure duplication.

Note the sites differ in what they capture: some want rendered text
(`TracingBuf`) and some want a warn count (`count_warnings`). Check which shape
each needs rather than assuming one helper fits all three; `extensions/git` and
`crates/cli` may need `ops-about` added to their dev-dependencies with
`features = ["test-support"]`.

**Origin**: discovered during TASK-1989 while fixing TASK-1735.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of the three sites uses ops_about::test_support (TracingBuf or count_warnings, whichever matches what it asserts) instead of a local Subscriber/MakeWriter plus its own pin_global_dispatcher
- [x] #2 No local copy of the set_global_default + rebuild_interest_cache pin remains at those three sites
- [x] #3 The tests at those sites still assert the same things and remain non-flaky under both cargo test (shared-process threads) and nextest
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resolved in TASK-2045 (wave183).

AC #1 substituted: the three sites now use the shared harness, but it lives in
`ops_core::test_utils` rather than `ops_about::test_support`. `ops-about`
depends on `ops-core`, and so do `crates/core` and `extensions/git`, so making
`ops-about` a dev-dependency of either would have inverted the layering with a
dev-dependency cycle (core -> about -> core). The harness moved down to
`ops-core` instead — `tracing-subscriber` is now an optional dependency there,
enabled by the existing `test-support` feature — and `ops_about::test_support`
re-exports `TracingBuf`, `WarnCounter`, `count_warnings`, `capture_warn`,
`capture_tracing` and `pin_global_dispatcher` from it, so every existing
`ops_about::test_support::*` import is unchanged. `extensions/git/src/config.rs`
was the site whose own comment predicted exactly this ("Collapsing the three
needs that dependency made optional-and-feature-gated instead").

AC #2: the workspace now has exactly one `set_global_default` +
`rebuild_interest_cache` pin, in `ops_core::test_utils::pin_global_dispatcher`.
The copies in `crates/core/src/test_utils.rs`, `crates/cli/src/test_utils.rs`
and `extensions/git/src/config.rs` are gone, together with the three local
`BufWriter` + `MakeWriter` shims.

AC #3: assertions are unchanged at all three sites. `cargo nextest run
--workspace --all-features` (2925 tests) and `cargo test --lib` (shared-process
threads) both green.
<!-- SECTION:NOTES:END -->
