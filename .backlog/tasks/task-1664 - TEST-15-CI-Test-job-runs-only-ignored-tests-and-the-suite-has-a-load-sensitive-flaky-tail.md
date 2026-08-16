---
id: TASK-1664
title: >-
  TEST-15: CI Test job runs only ignored tests, and the suite has a
  load-sensitive flaky tail
status: In Progress
assignee: []
created_date: '2026-08-15 00:00'
updated_date: '2026-08-16 09:42'
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
- [ ] #6 The remaining load-sensitive / environment-dependent tests are stabilised so a full run is reliably green
- [ ] #7 ops verify and ops qa pass
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
<!-- SECTION:NOTES:END -->
