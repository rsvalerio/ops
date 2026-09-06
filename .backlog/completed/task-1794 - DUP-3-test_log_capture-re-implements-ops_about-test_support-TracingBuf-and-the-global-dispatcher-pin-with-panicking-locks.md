---
id: TASK-1794
title: >-
  DUP-3: test_log_capture re-implements ops_about::test_support::TracingBuf and
  the global-dispatcher pin, with panicking locks
status: Done
assignee:
  - TASK-1995
created_date: '2026-08-27 11:24'
updated_date: '2026-08-28 20:26'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/tests.rs
  - extensions-rust/cargo-update/Cargo.toml
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/tests.rs:11-84` (`mod test_log_capture`), `:101-109`

**What**: The `test_log_capture` module hand-rolls a tracing-capture harness
that already exists in the workspace behind
`extensions/about/src/test_support.rs` (feature `test-support`):

| local (`tests.rs`) | shared (`extensions/about/src/test_support.rs`) |
|---|---|
| `struct BufWriter(Arc<Mutex<Vec<u8>>>)` + `Write` + `MakeWriter` (`:17-40`) | `struct TracingBuf(Arc<Mutex<Vec<u8>>>)` + `Write` + `MakeWriter` (`:34-69`) |
| `fn pin_global_dispatcher()` — `Once` + `set_global_default(sink)` + `rebuild_interest_cache()` (`:55-67`) | same pattern, tracked as DUP-3/TASK-1157 |
| `fn warn_breadcrumb_debug_escapes_control_characters` (`:101-109`) | `pub fn assert_debug_escapes_control_chars` (`:90`) |

The shared module's own doc comment records why it exists: three copies of this
exact `BufWriter(Arc<Mutex<Vec<u8>>>)` shim had already drifted, "style drift
between copies led to inconsistent log capture" (DUP-3 / TASK-1157 / TASK-0985).
This is a fourth copy. The `pin_global_dispatcher` half is a fifth copy of the
"install a global sink subscriber and swallow the error" idiom — the
`extensions-go/about` instance is already filed as TASK-1735, which lists the
other sites; this crate is not among them, so the finding is filed here against
this crate's file.

The copy is also **strictly worse** than the shared original in two ways:

- `BufWriter::contents` (`:21`) does `self.0.lock().unwrap()` where `TracingBuf::captured`
  does `.unwrap_or_else(std::sync::PoisonError::into_inner)`. A panicking test
  under `with_default` poisons the mutex and turns one failure into a cascade
  of unrelated failures.
- `contents` also does `String::from_utf8(..).unwrap()` where the shared helper
  uses `from_utf8_lossy` precisely because a subscriber flush can split a
  multi-byte character — a latent flake on any non-ASCII log field, and this
  crate logs `line = ?clean` from cargo output that the crate's own tests
  (`strip_ansi_round_trips_non_ascii`, `tests.rs:774`) prove can be non-ASCII.

**Why it matters**: DUP-3 with a correctness edge. Four copies of a harness
whose whole reason for being centralised is that copies drift, and this copy
has already drifted into the two failure modes the shared version was written
to avoid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 test_log_capture::BufWriter is replaced by ops_about::test_support::TracingBuf (dev-dependency ops-about with the test-support feature), or the shared module gains the capture_warn wrapper and this crate calls it
- [x] #2 warn_breadcrumb_debug_escapes_control_characters delegates to ops_about::test_support::assert_debug_escapes_control_chars instead of re-deriving the assertion
- [x] #3 No lock().unwrap() or String::from_utf8(..).unwrap() remains in the crate's tracing-capture path — poison is recovered and decoding is lossy
- [x] #4 The pin_global_dispatcher workaround lives in one shared place rather than a per-crate copy (coordinate with TASK-1735, which tracks the extensions-go copy)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-1995: the local `test_log_capture` module is gone. `ops_about::test_support` gained a shared `capture_warn` (built on TracingBuf, whose lock and UTF-8 handling recover instead of panicking) and `pin_global_dispatcher` was hoisted to one definition serving both `count_warnings` and `capture_warn`. cargo-update dev-depends on ops-about with the test-support feature.

AC #2 substitution: the test delegates to `assert_rendered_escapes_control_chars` — the shared half of `assert_debug_escapes_control_chars`, which now calls it — because the record is captured from a real parse and can carry no newline (see TASK-1783 notes). Nothing is re-derived in this crate.

AC #4: the dispatcher pin now lives only in ops_about::test_support. The extensions-go copy TASK-1735 tracked is already Done; the three remaining copies elsewhere are tracked by TASK-2014.
<!-- SECTION:NOTES:END -->
