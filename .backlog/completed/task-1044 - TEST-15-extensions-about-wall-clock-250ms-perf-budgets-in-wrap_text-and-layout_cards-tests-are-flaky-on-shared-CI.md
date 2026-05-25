---
id: TASK-1044
title: >-
  TEST-15: extensions/about wall-clock 250ms perf budgets in wrap_text and
  layout_cards tests are flaky on shared CI
status: Done
assignee: []
created_date: '2026-05-07 20:53'
updated_date: '2026-05-08 06:51'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `/Users/rsvaleri/projects/ops/extensions/about/src/text_util.rs:209-221` and `/Users/rsvaleri/projects/ops/extensions/about/src/cards.rs:365-381`

**What**:
Two unit tests assert wall-clock elapsed against a 250 ms budget:

- `text_util.rs::wrap_text_handles_very_long_input_in_linear_time` runs `wrap_text(...)` over a 10 000-token input and asserts `elapsed < Duration::from_millis(250)`.
- `cards.rs::layout_cards_handles_large_workspace` renders 150 cards into a grid and asserts `elapsed < Duration::from_millis(250)`.

Both fall under the TEST-15 prohibition on `sleep`/wall-clock-based assertions and the flakiness-pattern rules: a noisy CI runner (concurrent cargo test, slow VM, GH macOS shared host, `valgrind`/`miri`/coverage-instrumented runs) routinely takes 5–10x longer than the same code on a developer laptop. Both tests already comment that they are timing-as-shape proxies for asymptotic behaviour ("can't pin the asymptotic bound in a unit test cheaply"), which is exactly the situation TEST-15 calls out.

The existing TASK-1029 covers a separate 50 ms budget on `format_error_tail_does_not_decode_entire_buffer`; this is a distinct pair of sites in the about extension.

**Why it matters**:
A flaky perf-budget test under coverage / sanitiser / valgrind builds either (a) gets retroactively `#[ignore]`d (silent regression coverage loss, see TEST-26), or (b) produces noisy CI failures that train the team to ignore real regressions. The asymptotic property both tests defend (PERF-1 / TASK-0709 O(N) wrap, PERF-3 / TASK-0722 borrow-friendly layout) is real but not what a millisecond budget actually pins; replace with a structural or counting assertion (compare a 1k-input duration to a 10k-input duration ratio, or count allocations / clones via a Vec wrapper).

Suggested mitigations (any of):
- Use a ratio-based check: run the wrapper at 1k and 10k tokens and assert the 10k/1k ratio is < ~50 (true linear) instead of an absolute ms budget; the constant factor cancels and CI noise washes out.
- Count work units instead of time: add a debug-only counter (number of `display_width` calls in `wrap_text`, number of String allocations in `layout_cards`) and assert it is O(N) not O(N²).
- If the time bound stays, raise it to something CI-tolerant (≥2 s) and cfg-gate it behind `#[cfg(not(any(debug_assertions, sanitizer)))]` so debug / asan / coverage builds skip it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both tests no longer fail on a CI runner where wrap_text / layout_cards take >250 ms (e.g. coverage-instrumented build, qemu-emulated arch)
- [x] #2 The asymptotic property the tests are guarding (linear-time wrap, borrow-friendly layout) is still pinned by something a future regression must trip
- [x] #3 If retained as a wall-clock check, the budget is gated off under debug / sanitizer / coverage cfg so the TEST-15 flakiness surface is bounded
<!-- AC:END -->
