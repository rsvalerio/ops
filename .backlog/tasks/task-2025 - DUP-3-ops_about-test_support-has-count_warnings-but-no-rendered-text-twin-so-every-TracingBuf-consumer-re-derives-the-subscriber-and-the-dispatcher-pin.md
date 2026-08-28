---
id: TASK-2025
title: >-
  DUP-3: ops_about::test_support has count_warnings but no rendered-text twin,
  so every TracingBuf consumer re-derives the subscriber and the dispatcher pin
status: Triage
assignee: []
created_date: '2026-08-28 20:11'
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
- [ ] #1 ops_about::test_support exposes a capture_warnings<T>(f) -> (T, String) (or equivalent) that installs the fmt subscriber, pins the global dispatcher itself, and returns the rendered output
- [ ] #2 The subscriber configuration (max level, ansi) is decided once inside the helper rather than per call site
- [ ] #3 extensions-python/about's local capture_warns is deleted and its count_warnings(|| ()) pin workaround goes with it
- [ ] #4 The three remaining inline TracingBuf + fmt()::finish() sites route through the helper
<!-- AC:END -->
