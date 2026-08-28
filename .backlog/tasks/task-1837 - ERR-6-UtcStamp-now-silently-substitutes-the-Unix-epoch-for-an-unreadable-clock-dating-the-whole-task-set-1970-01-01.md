---
id: TASK-1837
title: >-
  ERR-6: UtcStamp::now silently substitutes the Unix epoch for an unreadable
  clock, dating the whole task set 1970-01-01
status: To Do
assignee:
  - TASK-2005
created_date: '2026-08-27 15:22'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/clock.rs
  - extensions/create-review-tasks/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/clock.rs:44-50`

**What**: `now()` collapses a clock failure into a sentinel value:

```rust
let secs = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_or(0, |d| d.as_secs());
Self::from_unix_secs(secs)
```

`duration_since(UNIX_EPOCH)` returns `Err(SystemTimeError)` whenever the host clock is before 1970 — a container started before NTP has stepped the clock, an RTC-less board that boots at 0, a VM restored from a snapshot with a bad clock. `map_or(0, …)` turns that `Err` into `0`, which `from_unix_secs` faithfully formats as `1970-01-01` / `00:00`. Nothing is logged, nothing is returned, and the signature (`fn now() -> Self`, infallible) leaves the caller no way to ask whether the stamp is real.

The consequences propagate through the whole run, because this one value fixes three things at once (lib.rs:99, 122-123):

- every task file's `created_date: '1970-01-01 00:00'` frontmatter;
- the main-task title `review-request-1970-01-01-<n>`;
- **the namespace `next_daily_sequence` allocates against** (lib.rs:187) — so the run allocates sequence 1 in a 1970 namespace that will still be empty on the next bad-clock boot, and two such runs on different real days both produce `review-request-1970-01-01-1`.

The module doc justifies a lot of arithmetic care ("the input contract is `SystemTime::now()`-shaped: non-negative", lines 20-23) but says nothing about the branch that manufactures that non-negativity by discarding an error.

**Why it matters**: The failure is invisible at exactly the moment the operator could act on it. A run against a mis-set clock produces a fully-formed, CLI-parseable task set carrying the wrong date, and the mis-dating is unrecoverable after the fact — the date is baked into the filename slug, the title, and the frontmatter of every file in the set. Compare the crate's handling of every *other* degraded condition: a missing `.backlog/tasks` dir, an unregistered provider, a malformed payload and an empty target list are each an `anyhow::bail!` naming the problem (lib.rs:111-117, 142-145). A broken clock is the one input that gets a silent default. Making `now()` return `anyhow::Result<UtcStamp>` (or `Option`) and letting `run_create_review_tasks` bail with "system clock reads before 1970-01-01; refusing to date review tasks" makes it consistent with the rest of the entry point, and costs one `?` at the single call site.

Note this is independent of TASK-1823 (TIME-1): adopting `chrono`/`jiff` replaces `from_unix_secs` and `civil_from_days` but does not by itself decide what `now()` does when the clock is unreadable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 UtcStamp::now no longer substitutes 0 for a duration_since(UNIX_EPOCH) error; it reports the failure to its caller
- [ ] #2 run_create_review_tasks surfaces an unreadable clock as an error naming the problem, in both RunMode::Write and RunMode::DryRun, instead of writing tasks dated 1970-01-01
- [ ] #3 a test drives the failure branch (e.g. via a from_unix_secs-level or injected-clock seam) and asserts no task file is created and the error names the clock
- [ ] #4 the clock.rs module doc states what now() does when the host clock precedes the Unix epoch
<!-- AC:END -->
