---
id: TASK-1823
title: >-
  TIME-1: create-review-tasks hand-rolls epoch-to-civil-date conversion in
  clock.rs on a false 'no time crate available' premise
status: Done
assignee:
  - TASK-2005
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 15:52'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/clock.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/clock.rs:1-95`

**What**: `clock.rs` implements the whole date pipeline by hand: `UtcStamp::from_unix_secs` derives the day number with `secs / 86_400` (line 25) and `civil_from_days` (line 68) open-codes Howard Hinnant's `civil_from_days` reduction — era/`doe`/`yoe` arithmetic, the 153-day month polynomial, and the March-based year shift — in 27 lines of `saturating_*` `u64` math. TIME-1 names exactly these constructs (`secs / 86_400` used to derive a date, hand-written epoch<->civil-date conversion, hand-written leap-year handling) as things that get written incorrectly by hand.

The module header justifies it with: "The workspace deliberately avoids `chrono` / `time`". That premise is false as stated. `cargo tree -i chrono --workspace` shows **chrono v0.4.45 is already compiled into the `ops` binary** via `duckdb -> arrow -> arrow-arith -> chrono` (and `jiff v0.2.35` arrives via `ops-tokei -> tokei -> env_logger`). Adding `chrono` as a direct dependency of this crate therefore costs zero additional compilation and zero new supply-chain surface — it is already built. Per the skill's classification notes, a documented justification earns one severity level down only when it is *specific and accurate*; this one is specific but factually wrong, so the baseline TIME-1 severity stands.

The reduction currently looks correct and is covered by six pinned timestamps plus leap-day, non-leap-century, and year-rollover cases, so this is not a live bug report. The cost is the standing maintenance liability: 27 lines of unchecked-by-the-compiler calendar arithmetic whose correctness rests entirely on a hand-written proof in a doc comment (lines 57-67), inside a crate whose actual job is writing backlog task files.

**Why it matters**: The output feeds `created_date` frontmatter and the `review-request-<date>-<n>` main-task title, and the title is the key `next_daily_sequence` allocates against. A wrong date silently produces a mis-dated task set and a sequence allocated in the wrong day's namespace — neither of which any test or gate would catch, because the only oracle for `civil_from_days` is the same table of six timestamps it was written against. Replacing it with `chrono` (already present) or `jiff` deletes the module, deletes the proof comment, and moves the correctness burden onto a maintained, fuzzed implementation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 clock.rs no longer derives a calendar date from secs / 86_400 or hand-rolls the civil_from_days reduction; the UTC date and HH:MM come from chrono (already in the binary's graph via duckdb -> arrow) or jiff
- [x] #2 UtcStamp keeps its pre-formatted (date, minutes) shape so backlog.rs render_task_file and the review-request title are byte-identical to today's output
- [x] #3 the existing from_unix_secs_formats_known_timestamps, day_after_leap_day_rolls_to_march, non_leap_century_skips_feb_29 and year_rolls_at_december_midnight cases still pass unchanged against the replacement
- [x] #4 if the hand-rolled implementation is deliberately kept instead, the module doc comment is corrected: it must not claim the workspace avoids chrono/time when chrono 0.4.45 and jiff 0.2.35 are already in the ops dependency graph
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
clock.rs now delegates the epoch -> civil-date reduction to `chrono` (added to
[workspace.dependencies] as `default-features = false, features = ["std"]`; it was
already compiled into the binary via duckdb -> arrow -> arrow-arith, and cargo-deny
licenses/bans/advisories stay green). `civil_from_days` and its hand-written
correctness proof are deleted; `UtcStamp` keeps its pre-formatted `(date, minutes)`
shape, so `render_task_file` output is byte-identical (the two golden shape tests
pass unchanged).

AC #3 substitution: `from_unix_secs` is now `-> Option<Self>` (chrono refuses rather
than clamps an unrepresentable instant), so the four pinned cases call a one-line
`stamp(secs)` test helper instead of `UtcStamp::from_unix_secs` directly. Every
timestamp and every expected value is unchanged. AC #4 does not apply: the
hand-rolled implementation was replaced, not kept.
<!-- SECTION:NOTES:END -->
