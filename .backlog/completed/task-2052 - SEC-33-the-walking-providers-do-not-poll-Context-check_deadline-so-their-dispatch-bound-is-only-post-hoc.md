---
id: TASK-2052
title: >-
  SEC-33: the walking providers do not poll Context::check_deadline, so their
  dispatch bound is only post-hoc
status: Done
assignee:
  - TASK-2060
created_date: '2026-08-29 13:03'
updated_date: '2026-08-29 18:09'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions-rust/loc/src/lib.rs
  - extensions/text-fixers/src/discovery.rs
  - crates/extension/src/data.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs`, `extensions-rust/loc/src/lib.rs`, `extensions/text-fixers/src/discovery.rs`

**What**: TASK-2017 put a wall-clock bound on provider dispatch in
`crates/extension/src/data.rs`: `DataRegistry::provide` installs a deadline on
the `Context` for the outermost provider, exposes it as
`Context::check_deadline()`, and converts an over-budget return into
`DataProviderError::TimedOut`. That makes every dispatch bounded *by
construction* and makes an overrun visible and attributable.

What it does not do is shorten the stall for a provider that never polls.
`provide` is synchronous and runs on the caller's thread, so the deadline is
cooperative: the tree-walking providers still run their walk to completion and
only then get told they were too slow. The three walkers that motivated the
finding — tokei (`lib.rs:228`), rust-loc, and text-fixers
(`discovery.rs:221`) — have not been converted. Each needs
`ctx.check_deadline()?` in its per-entry loop, which for text-fixers and
rust-loc means threading `&Context` (or the deadline) into the `walk` helpers
that currently take only a root path.

Separately, no cooperative check can interrupt a thread already blocked in a
syscall — a wedged NFS `readdir` still hangs the CLI. Bounding *that* means
running the dispatch off-thread or making the trait async, which is a breaking
change to `DataProvider` and was deliberately left out of TASK-2017's scope.

**Why it matters**: SEC-33 — resource exhaustion. The dispatch bound is real
but is currently a label on the stall rather than a cure for it in exactly the
providers the original finding named.

**Origin**: discovered during TASK-2047 while fixing TASK-2017; the walking
providers are outside that wave's file scope and the async/off-thread rework
exceeds it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The tokei, rust-loc and text-fixers walk loops call Context::check_deadline() once per entry and propagate the error
- [x] #2 A test drives one of those providers with a short Context budget and asserts it aborts before completing the walk
- [x] #3 Whether provider dispatch should run off-thread or the trait become async is decided and recorded, not left implicit
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in code-review/TASK-2060.

- tokei (`extensions/tokei/src/lib.rs`): `scan_tokei` / `collect_tokei` take
  `Option<&Deadline>` and check it once per **directory entry**, before the
  file-type test — the cheap-looking half of the walk (`read_dir`, `stat`) is
  the half that blocks on a wedged mount.
- rust-loc (`extensions-rust/loc/src/lib.rs`): the walk is `ignore`'s
  *parallel* one, whose per-entry closure cannot borrow the dispatch's
  `&mut Context`. Each worker polls a detached deadline, answers
  `WalkState::Quit` (which stops every worker, not just its own) and sets an
  `AtomicBool`; the flag is read after `run` joins the workers and turned back
  into the deadline's own `TimedOut`, so the failure still names the provider
  that owns the budget.
- New API in `ops-extension`: `Deadline` (public, `Clone + Send + Sync`) and
  `Context::deadline_handle()`, because both walkers live in free functions a
  `&Context` cannot reach. `Context::check_deadline` now delegates to
  `Deadline::check`, so there is one definition of the error.
- `From<anyhow::Error> for DataProviderError` now downcasts, so a walker's
  `TimedOut` survives the `anyhow` round trip these `collect_*` entry points
  impose instead of degrading into an opaque `ComputationFailed`. Cost pinned
  by test: anyhow's downcast searches the whole chain, so `.context(..)` added
  on top of a `DataProviderError` is dropped — documented as "don't do that".

AC #1 substitution — text-fixers. The AC's premise does not hold: **text-fixers
is not a data provider.** `discovery::walk` is reached only from
`runner::run_fixer`, driven by the `trailing-whitespace` /
`end-of-file-fixer` exec commands; no `Context` exists on that path and none
can, so `Context::check_deadline` is not callable there. Adding a private
timeout would invent a bound the provider budget is not, in a foreground
command the operator can already interrupt. Satisfied the AC's intent by
recording that analysis where the finding pointed
(`extensions/text-fixers/src/discovery.rs`, "No dispatch deadline here"), with
the condition for revisiting it. The two genuine walking providers are
converted.

AC #2: `a_spent_budget_aborts_the_tokei_walk_with_a_typed_timeout` and
`a_spent_budget_aborts_the_rust_loc_walk_with_a_typed_timeout` drive each
provider through `DataRegistry::provide` (the call that installs the deadline)
with a 1ns budget and assert a typed `TimedOut`; each pairs with a control run
over the same tree that produces the full record set, so the failure is the
deadline and not the fixture. `a_live_budget_leaves_the_*_intact` pins that an
unexpired deadline is a cancellation point and not a filter.

AC #3: decision recorded in the `DataProvider` trait docs
(`crates/extension/src/data.rs`, "Why `provide` stays synchronous and on the
caller's thread"). **Neither async nor off-thread.** Async is breaking for
every implementer, pulls a runtime into `ops-extension`, and buys nothing on
its own — the walkers are CPU/syscall-bound, so they would still need exactly
this per-entry check. Off-thread bounds the caller but not the work: a thread
blocked in `readdir` cannot be cancelled in Rust, so it leaks while still
holding resources and the process cannot exit — a visible stall traded for an
invisible one. The residual syscall-blocked case is therefore accepted, with
the revisit condition written down.
<!-- SECTION:NOTES:END -->
