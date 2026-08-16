---
id: TASK-1664
title: >-
  TEST-15: CI Test job runs only ignored tests, and the suite has a
  load-sensitive flaky tail
status: In Progress
assignee: []
created_date: '2026-08-15 00:00'
updated_date: '2026-08-16 12:20'
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
<!-- SECTION:NOTES:END -->
