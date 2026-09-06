---
id: TASK-2025
title: >-
  DUP-3: ops_about::test_support has count_warnings but no rendered-text twin,
  so every TracingBuf consumer re-derives the subscriber and the dispatcher pin
status: Done
assignee:
  - TASK-2045
created_date: '2026-08-28 20:11'
updated_date: '2026-08-29 13:52'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/about/src/test_support.rs
  - extensions-python/about/src/lib.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/query.rs
  - extensions-terraform/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/test_support.rs` (the `tracing_capture` / `level_counter` modules), with the duplicated sequence at `extensions-python/about/src/lib.rs` (`capture_warns`), `extensions-rust/about/src/coverage_provider.rs:224`, `extensions-rust/about/src/query.rs:958`, and `extensions-terraform/about/src/lib.rs:641`.

**What**: `test_support` exposes two halves that do not compose. `count_warnings` handles the `Interest`-cache pin correctly but only returns a count; `TracingBuf` returns rendered text but leaves every consumer to write the same six lines by hand:

```rust
let buf = TracingBuf::default();
let subscriber = tracing_subscriber::fmt()
    .with_writer(buf.clone())
    .with_max_level(tracing::Level::WARN)
    .with_ansi(false)
    .finish();
let out = tracing::subscriber::with_default(subscriber, f);
```

There is no `capture_warnings<T>(f) -> (T, String)` to call, so each site also has to rediscover the global-dispatcher hazard that `count_warnings` documents. TASK-1992 hit exactly that: the new `colliding_url_keys_*` test captured an empty buffer at random until `capture_warns` was made to call `count_warnings(|| ())` first purely for its `pin_global_dispatcher` side effect — a workaround that only works because the pin is a process-wide `Once`, and which reads as nonsense at the call site.

Each consumer also picks its own `with_max_level` and `with_ansi`, so what "captured output" means drifts between crates.

**Why it matters**: the harness was extracted (DUP-3 / TASK-0985, TASK-1157, TASK-1735) precisely so tracing assertions would have one shape. The missing entry point means the *most common* use — assert on the rendered warn — is still copy-pasted four times, and the one hazard the module documents is reachable only through a side effect of an unrelated function. A `capture_warnings` twin sitting next to `count_warnings`, pinning the dispatcher itself, collapses all four copies and makes the hazard unreachable by construction.

Related but distinct: TASK-2014 covers replacing *hand-rolled* subscribers with `test_support`; this covers the gap in `test_support` those replacements will land on.

**Origin**: discovered during TASK-1992 while fixing TASK-1756 and TASK-1757.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops_about::test_support exposes a capture_warnings<T>(f) -> (T, String) (or equivalent) that installs the fmt subscriber, pins the global dispatcher itself, and returns the rendered output
- [x] #2 The subscriber configuration (max level, ansi) is decided once inside the helper rather than per call site
- [x] #3 extensions-python/about's local capture_warns is deleted and its count_warnings(|| ()) pin workaround goes with it
- [x] #4 The three remaining inline TracingBuf + fmt()::finish() sites route through the helper
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-1995 added `ops_about::test_support::capture_warn`, the rendered-text twin of `count_warnings`, with the dispatcher pin hoisted to a single definition shared by both. The remaining work here is migrating the other TracingBuf consumers onto it.

Resolved in TASK-2045 (wave183).

AC #1 / #2 were already satisfied by TASK-1995's `capture_warn`; TASK-2045
moved that helper (and the whole harness) down into `ops_core::test_utils`, so
`ops_about::test_support` now re-exports it — see TASK-2014's notes for why the
harness could not stay in an extension crate. The subscriber configuration
(WARN, ANSI off) is decided once inside the helper. A level-parameterised
`capture_tracing<F, R>(level, f) -> (String, R)` sits underneath it for the
call sites that need the closure's return value or a non-WARN level.

AC #3: `extensions-python/about/src/lib.rs`'s local `capture_warns` is deleted,
and with it the `count_warnings(|| ())` call that existed purely for its
`pin_global_dispatcher` side effect. Its four call sites use `capture_tracing`
directly. The crate's now-unused `tracing-subscriber` dev-dependency is gone.

AC #4 partially substituted — two of the four sites the task enumerated no
longer exist in the shape it describes:
- `extensions-rust/about/src/query.rs` has been deleted from the tree since the
  finding was filed; there is nothing left to migrate.
- `extensions-terraform/about/src/lib.rs` already routes through `capture_warn`.
The remaining inline sites, both in `extensions-rust/about/src/coverage_provider.rs`,
now use `capture_tracing`. A third site in that file
(`project_coverage_warn_fires_once_under_concurrent_first_callers`) keeps its
own subscribers on purpose: it installs one *per spawned thread* over a shared
`TracingBuf`, which is the one shape a thread-local helper cannot provide. It
previously had no dispatcher pin at all — a latent instance of the flake this
task is about — and now calls the newly-public
`test_support::pin_global_dispatcher()` itself, which is the sanctioned
replacement for the `count_warnings(|| ())` nonsense.
<!-- SECTION:NOTES:END -->
