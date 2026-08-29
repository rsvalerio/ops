---
id: TASK-2058
title: >-
  DUP-3: seven TracingBuf consumers still hand-roll the fmt subscriber that
  capture_tracing now owns
status: To Do
assignee:
  - TASK-2061
created_date: '2026-08-29 13:54'
updated_date: '2026-08-29 17:27'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/about/src/deps_provider.rs
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/tests/payload_cap.rs
  - extensions-rust/create-review-tasks/src/provider.rs
  - extensions-rust/deps/src/test_support.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/deps_provider.rs:63`, `extensions-rust/about/src/members.rs:292`, `extensions-rust/about/src/units.rs:312`, `extensions-rust/about/src/manifest_cache.rs:609`, `extensions-rust/metadata/src/ingestor.rs:444`, `extensions-rust/metadata/src/tests/payload_cap.rs:180`, `extensions-rust/create-review-tasks/src/provider.rs:432`, `extensions-rust/deps/src/test_support.rs`

**What**: TASK-2045 (wave183) finished the harness consolidation TASK-2014 /
TASK-2025 asked for: `ops_core::test_utils` now owns `TracingBuf`, the
`WarnCounter`, the single global-dispatcher pin, and the two entry points
`capture_tracing(level, f) -> (String, R)` and `capture_warn(f) -> String`,
re-exported by `ops_about::test_support`. Those two tasks only enumerated the
sites they had found, so seven consumers were left outside their scope and
still open-code the same six lines:

```rust
let buf = TracingBuf::default();
let subscriber = tracing_subscriber::fmt()
    .with_writer(buf.clone())
    .with_max_level(tracing::Level::WARN)
    .with_ansi(false)
    .finish();
let out = tracing::subscriber::with_default(subscriber, f);
```

`extensions-rust/deps/src/test_support.rs` is a further variant: it keeps its
own `BufWriter` + `MakeWriter` scaffold (with panicking locks) rather than
using `TracingBuf` at all.

**Why it matters**: DUP-3, and the same silent-flake class the harness exists
to close. None of these sites pins the global dispatcher, so each is one
parallel first-hit away from `tracing` caching `Interest::never()` for its warn
callsite and the capture coming back empty at random. Routing them through
`capture_tracing` removes the copy and makes the hazard unreachable, exactly as
it did for the sites TASK-2014 and TASK-2025 covered. Each also picks its own
`with_max_level` / `with_ansi`, so "captured output" still means slightly
different things across the crates.

**Origin**: discovered during TASK-2045 while fixing TASK-2025.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the seven inline TracingBuf + tracing_subscriber::fmt() sites uses ops_about::test_support::capture_tracing (or capture_warn) instead
- [ ] #2 extensions-rust/deps/src/test_support.rs drops its private BufWriter/MakeWriter scaffold in favour of the shared harness
- [ ] #3 No capture site is left without a global-dispatcher pin; assertions at each site are unchanged
<!-- AC:END -->
