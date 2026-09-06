---
id: TASK-1664
title: >-
  TEST-15: CI Test job runs only ignored tests, and the suite has a
  load-sensitive flaky tail
status: Done
assignee: []
created_date: '2026-08-15 00:00'
updated_date: '2026-08-17 20:08'
labels:
  - ci
  - testing
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/ci.yml` (Test job)

**What**: the job ran

```yaml
run: cargo test --all --all-features -- --ignored
```

`-- --ignored` runs **only** ignored tests. Measured on 2026-08-15:

| | Tests run | Filtered out |
|---|---|---|
| CI's Test job | **22** | **2476** |
| A normal run | ~2498 | 0 |

So the required **Test** status check has been passing on roughly 1% of the
suite since it was written. `--include-ignored` runs both sets.

**Why it matters**: every green Test check on every PR has been close to
meaningless. It is also self-concealing — the suite drifted while nothing was
watching, so turning the flag on surfaces a backlog of pre-existing failures
rather than a clean pass.

**What turning it on revealed.** Running the full suite locally exposed several
tests that pass in isolation and fail when the suite runs in parallel. Root
cause in most cases is a **wall-clock budget asserted in a debug build**: fine
on an idle machine, blown under CPU contention, which is the normal state of a
shared CI runner.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CI runs the full test suite, not only ignored tests
- [x] #2 `expand::tests::from_env_hit_path_avoids_canonicalize_syscall` no longer asserts on wall-clock time
- [x] #3 `command::tests::expand::...::expand_to_leaves_microbench_does_not_regress` no longer asserts on wall-clock time
- [x] #4 `command::tests::exec::...::emit_output_events_shares_buffer_across_lines` no longer asserts on wall-clock time
- [x] #5 `tmpdir_swap_after_from_env_is_not_observed` no longer breaks concurrent tempfile users
- [x] #6 The remaining load-sensitive / environment-dependent tests are stabilised so a full run is reliably green
- [x] #7 ops verify and ops qa pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Done

`ci.yml`: `-- --ignored` → `-- --include-ignored`.

Four tests converted from timing to behaviour. Each conversion was verified to
still catch the regression it guards, by reintroducing that regression and
confirming a failure:

- **`from_env_hit_path_avoids_canonicalize_syscall`** — was 200k calls under a
  1s budget (measured 2.0–4.7s under load). Now counts `std::fs::canonicalize`
  calls via a **path-keyed** seam and asserts zero on the warm path. Forcing a
  canonicalize before the cache probe fails with `saw 1000 syscall(s)`.
  Path-keyed rather than a global counter because test binaries run in parallel
  and unrelated tests bumped a global one (observed spurious delta of 3).
- **`expand_to_leaves_microbench_does_not_regress`** — was 1k expansions under a
  2s budget (measured 9.8s under load). Now counts store walks via a
  **thread-local** seam and asserts exactly one per visited node (551).
  Reintroducing the TASK-0766 `canonical_id` + `resolve` double lookup reports
  1102 and fails. Note the old budget could not have caught that 2x regression
  anyway — it only ever caught catastrophic slowdowns.
- **`emit_output_events_shares_buffer_across_lines`** — the wall-clock half was
  **deleted**, not converted. Its own comment conceded it detected nothing
  ("the pre-fix per-line String allocation passed too, so this is a sanity
  floor, not a precision regression detector") while being the single most
  reliable failure once tests run in parallel. The pointer-identity assertion
  beside it is the real detector and is untouched.
- **`tmpdir_swap_after_from_env_is_not_observed`** — set `TMPDIR` to a
  non-existent path. `set_var` is process-global while `#[serial]` only
  serialises against other `#[serial]` tests, so for the duration of the window
  every concurrent test calling `tempfile::tempdir()` failed — ~85 candidates in
  `ops-core` alone; `stack::tests::detect_finds_cargo_toml` was caught dying
  this way. Now swaps to a real tempdir, which proves the same contract.

### Remaining — AC #6

> **Superseded — see "AC #6 closeout" below.** Every row in the table here was
> recorded from local runs before TASK-1665 landed, and each has since been
> fixed or shown to be already-handled. Kept for the history of what the flag
> flip surfaced; do not read it as current state.

A full run is **not yet reliably green**. Over five full-suite runs with no
artificial load, one round passed clean (2498/0) and the others failed 2–3
tests, with a rotating cast:

| Test | Suspected cause |
|---|---|
| `cards::tests::layout_cards_handles_large_workspace` | wall-clock budget |
| `probe::cargo::...::cargo_builtins_list_is_in_sync` | environment-dependent (installed cargo) |
| `tests::check_cargo_tool_installed_fmt` | environment-dependent (tool presence) |
| `tests::check_tool_status_simple_installed` | environment-dependent (tool presence) |
| `stack::tests::detect_finds_ansible` | env/tempdir race |
| `query::tests::typed_manifest_cache_recovers_from_poison_with_warn` | thread scheduling, only seen under extreme load |

Four wall-clock assertions remain workspace-wide
(`extensions-rust/tools/src/probe/timeout.rs`,
`crates/runner/src/command/tests/parallel_infra.rs`,
`crates/runner/src/command/tests/exec.rs`, `crates/core/src/subprocess/drain.rs`)
— those are timeouts/drains where a duration is the thing under test, so they
need judgement rather than blanket conversion.

The environment-dependent ones are a separate class and may behave differently
on a CI runner than on a workstation — worth reading the actual CI result before
guessing at them.

**Consequence for AC #1**: with the flag flipped, the Test check is expected to
be intermittently red until AC #6 lands. That is strictly better information
than a green check over 1% of the suite, but it is not a clean state and should
not be left sitting.

### ops-about wall-clock sites (folded in from TASK-1667, archived as a duplicate)

Hit while running the gate for TASK-0137/0165/1567: `ops qa` failed once with
`-p ops-about --lib`, then passed on re-run with no code change. Filed as
TASK-1667 before spotting that AC #6 here already owns it — archived, with its
site inventory kept below because it names two sites the table above does not.

All three assert a *timing ratio* between a small and a large input to prove
O(N) behaviour, so the denominator being scheduled out inflates the ratio past
the threshold. The assertion measures the scheduler, not the algorithm:

| Site | Shape |
|---|---|
| `extensions/about/src/cards.rs:394-407` | `layout_cards_in_grid_with_width`, asserts `ratio < 20.0` — this is the `layout_cards_handles_large_workspace` row in the table above |
| `extensions/about/src/text_util.rs:336-349` | same ratio shape — **not** in the table above |
| `extensions/about/src/manifest_cache.rs:398-415` | two `Instant::now()` measurements — **not** in the table above |

Same conversion as the four already done: count the inner-loop operations
through a seam and assert linear growth, rather than inferring it from elapsed
time. Note `cards.rs` and `text_util.rs` are the same helper shape, so one
conversion likely covers both.
### AC #6 progress: the OPS_ROOT poison breadcrumb test (separate PR)

Found while running CI for the lint-policy PR (#21): `Test` failed on
`expand::tests::ops_root_cache_len_surfaces_poison_breadcrumb` and failed again
on rerun, while main stayed green. It was **not** caused by that PR — the only
behavioural changes it makes to `ops-core` are `from_secs(180)`→`from_mins(3)`
(same value), `with_path` taking `&io::Error`, and one test constant;
`expand.rs` got doc comments only and `sync.rs` was untouched.

**Root cause.** `sync::lock_recover_warn` calls `Mutex::clear_poison` before
warning, so a poisoned lock yields exactly *one* breadcrumb — whichever caller
recovers first consumes it. The test poisoned the process-global `OPS_ROOT`
cache and then asserted on the breadcrumb, so it was racing every other test
that reaches that cache.

**Why the obvious fix does not work.** The cache tests sat in
`#[serial(ops_root_cache)]` while the env tests sat in the default `#[serial]`
group, and those two groups do not serialise against each other — so the first
attempt was to merge the groups. That is insufficient: of 27 tests in
`expand`, **26 reach the cache and 15 of those are not serialised at all**
(every `test_vars()` caller goes `test_vars` → `from_env` →
`cached_ops_root_arc`). Measured, and confirmed by the merged-group build still
failing under load on run 3 of 5.

**Fix.** Deleted the racing test and moved its contract to `crates/core/src/sync.rs`,
tested against a **stack-local** `Mutex` poisoned via `thread::scope` — nothing
else can reach that lock, so the assertion cannot race. Three tests now cover
the seam: the breadcrumb names the site, a healthy lock stays silent, and
`lock_recover` recovers without warning and clears the poison.

Held to this task's standard: verified the new test catches the regression it
guards by deleting the `tracing::warn!` and confirming
`lock_recover_warn_emits_breadcrumb_naming_the_seam` fails, then restoring it.
20 consecutive `ops-core` runs under 16-core load: 0 failures (the old test
reproduced locally within 5).

Net coverage: `ops-core` 325 → 327 tests. AC #6 is **not** complete — the
environment-dependent tests in the table above are untouched.

### Second instance: typed_manifest_cache in ops-about-rust

The `ops-core` fix above exposed the next one. `Test` on PR #22 failed on
`query::tests::typed_manifest_cache_recovers_from_poison_with_warn` — a
different crate, and the test this task's table already lists as
"thread scheduling, only seen under extreme load". PR #22 does not touch
`ops-about` at all (its diff is `crates/core/src/{expand,sync}.rs` plus this
file), so it is pre-existing.

Same root cause, different shape. `lock_typed_manifest_cache` recovers via
`clear_poison()`, so the breadcrumb is one-shot. The failing assertion was the
*premise* check — `"mutex must be poisoned for the test premise"` — i.e.
something had already recovered the lock before the test looked.

The wrinkle: all 10 cache tests in `query.rs` **were** correctly serialised
under `#[serial(typed_manifest_cache)]`. The racers are in sibling modules that
reach the same static *indirectly*, through a provider's `provide()` →
`load_workspace_manifest`: 13 tests in `identity/mod.rs` and 1 in `units.rs`,
none serialised. Same test binary, so they interleave freely.

Fix here is the serial group rather than the local-mutex rewrite used in
`ops-core`, because the seam does not decompose the same way:
`lock_typed_manifest_cache` takes `&'static Mutex<TypedManifestCache>` and its
`recovery_count` lives in a process-global `AtomicU64`, so a stack-local mutex
would not make the count-based assertions deterministic either. Added the
attribute to all 14, bringing every cache-reaching test to 25/25 serialised,
and documented the invariant at `typed_manifest_cache()` so a future test added
to `identity/mod.rs` does not silently reopen the race.

Caveat on evidence: this one does **not** reproduce locally (0/10 failures on 16
cores before the change, 0/10 after), so unlike the `ops-core` fix the load
test cannot distinguish. CI on a 2-core runner is the real check.

### AC #6 progress: the three ops-about wall-clock sites

All three sites from the inventory above are converted. Each conversion was
verified the way this task requires — reintroduce the regression it guards and
confirm the new assertion fails:

- **`text_util.rs` `wrap_text`** — was a growth ratio (10x input under 20x
  time). Replaced by a thread-local seam counting the characters handed to
  `display_width`, asserted **exactly**: every word once, every emitted line
  once, derived from the actual output rather than hardcoded. Reintroducing the
  TASK-0709 `display_width(&current_line)` call reports 80,621 characters
  against 14,300.
- **`cards.rs` `layout_cards_in_grid_with_width`** — extracted `row_parts` and
  asserted the borrow contract by pointer identity, mirroring
  `emit_output_events_shares_buffer_across_lines`. Reintroducing the per-cell
  `.cloned()` fails on the pointer comparison. The surviving
  `layout_cards_handles_large_workspace` now asserts grid *shape* (3 cards per
  row at 120 columns, one blank separator, none trailing) with no timing.
- **`manifest_cache.rs`** — the 5-second bound was **deleted**, not converted,
  for the reason its own comment gave: it was a deadlock smoke check, and a
  deadlock presents as `join` never returning, not as slow elapsed time. The
  threads completing is the evidence; the `Arc::ptr_eq` assertions are the
  detector.

**Two findings worth recording**, because they change how the remaining sites
should be handled:

1. **Both ratio assertions were guarding constant-factor regressions, which no
   growth ratio can detect at any bound.** Re-measuring `current_line` is capped
   by `max_width`, and a per-cell clone is still linear — each just moves the
   constant. Measured: with the quadratic shape restored, the 10x-input ratio
   came out 0.09% away from passing. The TASK-1152 note ("tightened 50x → 20x
   to catch 2-3x regressions") was mistaken about what the shape could catch.
   Assert the count exactly; do not tune a bound.
2. **`wrap_text`'s O(N^2) framing was wrong** and is corrected in the source
   comment. The pre-fix cost was `O(N * max_width)` — linear, 5.6x the constant.

Also: with `max_lines` breaking the word loop after 50 lines, the old test's 1k-
and 10k-word inputs consumed the same ~800 words, so the two sizes it compared
did the same work. The new test sets `max_lines` past the line count so the
whole input is measured.

Evidence: `ops-about` 107 tests, **20/20 consecutive runs green under 16-core
load** (`yes` on every core). Suite wall-clock dropped from timing-dominated to
0.15s.

### Third instance, and a new one: the ops-git tracing capture

Found by running the full workspace suite 3x under 16-core load after the
conversions above: run 3 failed on
`config::tests::read_origin_url_warns_on_control_byte_drop_keeping_prior_valid`
(`extensions/git/src/config.rs`), which is **not in the table above** — a new
member of the flaky tail. Reproduced at 1/30 under load.

**Root cause: `tracing`'s process-global callsite interest cache.** The parser
assertion passed; only the captured buffer came back empty. `tracing` caches
each callsite's `Interest` process-wide, computed against the dispatchers
registered when that callsite is first hit. This test installed a *scoped*
(`with_default`) subscriber only, so a parallel test thread that first-hits the
same callsite with no dispatcher on its thread registers `Interest::never()`
globally — and the `warn!` then short-circuits before consulting our
thread-local subscriber.

**This is already solved twice in this repo** and the ops-git test simply
open-coded the capture scaffold without the fix: `ops_core::test_utils` and
`ops_cli::test_utils` both carry a `pin_global_dispatcher()` that installs one
permanently-registered sink and calls `rebuild_interest_cache()`, with the mechanism
written up in `core/src/test_utils.rs`. Added the same pin to the ops-git test.

**Evidence, both directions and deterministic** — not just a load-run count.
The 1/30 base rate is too low to prove anything by repetition, so the race was
made deterministic: install the scoped subscriber, then hit the same callsite
from a spawned thread (which is what a parallel test does by accident), then
emit. Without the pin that fails every time with the identical "empty capture"
symptom; with the pin it passes every time. The scratch test was removed once
both directions were confirmed. Load runs after the fix: **0/40** on 16 cores.

Note the first attempt at a deterministic repro poisoned the callsite *before*
installing the subscriber and did **not** fail — setting the first scoped
dispatcher rebuilds the interest cache, which clears an earlier `never`. Only a
poisoning that lands *after* the subscriber is installed sticks. Worth knowing
before writing any similar test.

**Follow-up worth considering (not done here):** this is now the third copy of
`pin_global_dispatcher`. Collapsing them needs `tracing-subscriber` made an
optional, `test-support`-gated dependency of `ops-core` rather than a
dev-dependency — the module doc in `core/src/test_utils.rs` currently
documents the exclusion as deliberate for that reason. That is a change to a
shared crate's stability contract, so it wants a decision rather than being
folded into a flake fix.

### AC #6 closeout: reading the CI history, and the table was stale

The note above said to read a real CI result before guessing at the
environment-dependent tests. Done — every failed CI run since the flag flip
(`gh run list --workflow CI`, 7 failures, all 2026-08-09..16):

| Run | Test that failed | Status now |
| --- | --- | --- |
| 31902029618 | `cargo_builtins_list_is_in_sync` | fixed by TASK-1665 |
| 31902399577 | `check_cargo_tool_installed_honours_cargo_env` | fixed by TASK-1665 |
| 31902760742 | `command::tests::exec::run_plan_echo_success` | fixed by TASK-1664 |
| 31944268138, 31944676883 | `ops_root_cache_len_surfaces_poison_breadcrumb` | fixed above |
| 31946364881 | `typed_manifest_cache_recovers_from_poison_with_warn` | fixed above |
| 31320181114 | (build failure, not a test) | n/a |

Two things fall out of this.

**The table in "Remaining — AC #6" was stale.** Its entries were recorded from
local runs before TASK-1665 landed. Current state of each: `cargo_builtins_
list_is_in_sync` no longer shells out at all (it asserts on the parser);
`check_cargo_tool_installed_fmt` and `check_tool_status_simple_installed` are
`#[ignore = "requires rustup + cargo-fmt installed"]` with the requirement
documented per TEST-24; `detect_finds_ansible` was collateral damage from the
`TMPDIR` swap fixed earlier in this task. None needed further work.

**The cargo-colour bug was a live production defect, not a test problem.**
`cargo --list` output is ANSI-wrapped when `CARGO_TERM_COLOR=always` (which
this repo's CI sets), the whitespace-splitting parser then reads
`\x1b[1m\x1b[96madd` as the subcommand name, and every cargo tool is reported
as not installed. Any user with that variable exported got wrong answers from
`ops`. It surfaced only because CI started running the full suite — which is
the clearest argument available that AC #1 was worth doing.

### AC #6: the four remaining wall-clock sites, judged

The note above said these need judgement rather than blanket conversion. Each
was read against what its assertion can actually detect:

- **`crates/runner/src/command/tests/exec.rs`** — already converted (TASK-1664):
  `as_millis() > 0` required the step to be at least a millisecond *slow*;
  now `as_nanos() > 0`, which pins the invariant worth having (the duration was
  measured at all). This is the site CI run 31902760742 failed on.
- **`extensions-rust/tools/src/probe/timeout.rs`** — **kept as-is.** The
  duration genuinely is the contract (ASYNC-6: the wrapper honours its
  deadline), and the separation is real: `sleep 30` behind a 1-second timeout,
  bounded at 10 seconds. Correct behaviour lands near 1s, a broken deadline
  near 30s. This is what a well-formed timing assertion looks like.
- **`crates/runner/src/command/tests/parallel_infra.rs`** — timing assertion
  **deleted** as redundant. If abort never fired, task B falls through to its
  5s sleep and returns `success`, which the harvest assertion already rejects;
  `elapsed < 4s` could only fail alongside an assertion that states the
  contract directly.
- **`crates/core/src/subprocess/drain.rs`** — timing assertion **deleted**,
  because it could not fail for any reason except load. It had no resolution
  for the "per-8 KiB user-space spin" it named (2048 iterations over an
  in-memory cursor is milliseconds, not seconds); it could not catch
  non-termination either, since `elapsed` is read only after `read_capped`
  returns; and there was never a shipped slow path to regress from — the
  `io::copy` discard and the test landed in the same commit (524af94). A
  counting seam is not available here: wrapping the reader would defeat the
  `BufRead` specialisation that makes `io::copy` fast, so measuring would
  change what is measured. The byte accounting is the contract that survives.

Net: three of the four wall-clock sites named in this task are gone, and the
one that remains earns its assertion.

Evidence: 6 consecutive full-workspace runs under 16-core load, all green.
<!-- SECTION:NOTES:END -->
