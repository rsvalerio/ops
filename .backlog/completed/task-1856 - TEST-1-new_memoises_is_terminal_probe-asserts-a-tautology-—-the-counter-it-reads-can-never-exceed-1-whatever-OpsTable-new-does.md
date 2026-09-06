---
id: TASK-1856
title: >-
  TEST-1: new_memoises_is_terminal_probe asserts a tautology — the counter it
  reads can never exceed 1, whatever OpsTable::new does
status: Done
assignee:
  - TASK-1984
created_date: '2026-08-27 15:28'
updated_date: '2026-08-29 00:37'
labels:
  - code-review-rust
  - testing
dependencies: []
modified_files:
  - crates/core/src/table.rs
  - crates/core/src/style.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/table.rs:165-182` (the test), `crates/core/src/style.rs:36-64` (the counter it reads)

**What**: The test claims to pin PERF-3 / TASK-1439 — that repeated `OpsTable::new` calls do not re-invoke `stdout().is_terminal()`:

```rust
    #[test]
    fn new_memoises_is_terminal_probe() {
        let before = crate::style::stdout_is_terminal_probe_count();
        for _ in 0..16 {
            let _ = OpsTable::new();
        }
        let after = crate::style::stdout_is_terminal_probe_count();
        assert!(
            after - before <= 1,
            "stdout is_terminal probed {} times across 16 constructions; expected ≤1",
```

The counter is incremented **inside** the `OnceLock::get_or_init` closure, and nowhere else in the file (`fetch_add` appears exactly once, at style.rs:41):

```rust
pub fn stdout_is_terminal() -> bool {
    static STDOUT_TTY: OnceLock<bool> = OnceLock::new();
    *STDOUT_TTY.get_or_init(|| {
        STDOUT_PROBES.fetch_add(1, Ordering::Relaxed);
        std::io::stdout().is_terminal()
    })
}
```

`OnceLock::get_or_init` runs its closure at most once per process by construction, so `STDOUT_PROBES` can never exceed 1 — `after - before` is 0 or 1 no matter what `OpsTable::new` does. The counter measures the `OnceLock`, not the number of `isatty` calls.

The regression it names is TASK-1439's pre-fix state: `OpsTable::new` calling `std::io::stdout().is_terminal()` **directly** instead of routing through the cache. Reintroduce exactly that and the counter stays at 0, `after - before == 0`, and the test still passes. It cannot observe the defect it exists for.

**Why it matters**: TEST-1 / TEST-11. A test that cannot fail is worse than no test — it reports coverage of a property nothing is checking, so the next person to touch `OpsTable::new` gets a green suite for a change that undoes the memoisation. It also leaves `style::stdout_is_terminal_probe_count` as a `pub` (`#[doc(hidden)]`) production API, plus a process-lifetime `AtomicUsize`, paying rent for a test that proves nothing.

To actually pin the contract the probe has to be counted at the *call site* rather than inside the cache — e.g. an injectable probe seam that `OpsTable::new` and `style::color_enabled` both go through, with the test counting invocations of the seam.

<!-- scan confidence: verified by reading; `fetch_add` occurs exactly once in style.rs, inside the get_or_init closure at line 41 -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The memoisation test fails when OpsTable::new is changed to call std::io::stdout().is_terminal() directly instead of routing through style::stdout_is_terminal
- [x] #2 The probe is counted where it is called rather than inside OnceLock::get_or_init, so the counter reflects isatty invocations rather than cache initialisations
- [x] #3 If no meaningful seam is worth adding, the test and the pub #[doc(hidden)] stdout_is_terminal_probe_count API plus its AtomicUsize are removed rather than left as false coverage
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-1984. The counter moved out of the OnceLock::get_or_init closure to the top of `style::stdout_is_terminal`, so it now counts calls to the shared accessor rather than cache initialisations; renamed STDOUT_PROBES -> STDOUT_TTY_QUERIES and the seam `stdout_is_terminal_probe_count` -> `stdout_tty_query_count` (only caller is the test). The test is rewritten as `new_routes_tty_probe_through_shared_cache` and asserts the delta is >= 16 across 16 constructions (>= not == because sibling tests in the same binary legitimately consult the same accessor in parallel). AC#1 verified empirically: temporarily changing OpsTable::new to call std::io::stdout().is_terminal() directly makes the test FAIL, where the old tautological assertion passed. The seam is therefore kept rather than removed (AC#3 alternative not needed).
<!-- SECTION:NOTES:END -->
