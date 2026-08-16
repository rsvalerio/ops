---
id: TASK-1667
title: 'TEST: ops-about wall-clock ratio assertions flake under parallel load'
status: Triage
assignee: []
created_date: '2026-08-15 21:21'
labels:
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ops qa` failed once with `-p ops-about --lib`, then passed on re-run with no code change. The cause is the same wall-clock-assertion pattern that commit 98e9ef6 ("test(core,runner): replace wall-clock assertions with behavioural seams") removed from core and runner — ops-about still has it.

Sites, all asserting a *timing ratio* between a small and a large input to prove O(N) behaviour:
- extensions/about/src/cards.rs:394-407 — `layout_cards_in_grid_with_width`, asserts `ratio < 20.0`
- extensions/about/src/text_util.rs:336-349 — same shape
- extensions/about/src/manifest_cache.rs:398-415 — two `Instant::now()` measurements

Under a loaded machine (a full `cargo test --workspace --all-features` run saturates cores) the small-input measurement can be scheduled out and inflate the denominator's ratio past the threshold. The assertion measures the scheduler, not the algorithm.

Fix in the style of 98e9ef6: replace the timing measurement with a behavioural seam — e.g. a counter of the inner-loop operations, asserted to grow linearly — so the property under test is observed directly instead of inferred from elapsed time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No test in ops-about asserts on elapsed wall-clock time or a ratio of two elapsed times
- [ ] #2 The O(N) properties currently covered by those tests are still covered, via a counter or other deterministic seam
- [ ] #3 cargo test -p ops-about --all-features passes repeatedly under parallel load
<!-- AC:END -->
