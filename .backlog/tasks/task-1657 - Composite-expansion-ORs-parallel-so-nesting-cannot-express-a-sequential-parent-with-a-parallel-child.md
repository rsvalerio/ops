---
id: TASK-1657
title: >-
  Composite expansion ORs parallel, so nesting cannot express a sequential
  parent with a parallel child
status: Done
assignee: []
created_date: '2026-08-06 14:30'
updated_date: '2026-08-15 00:00'
labels:
  - design
  - runner
  - config
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Root cause behind TASK-1656.** Filed separately because that one has a one-line workaround, while this is a semantics decision.

**File**: `crates/runner/src/command/resolve.rs:265-294` (`expand_inner`)

**What**: composites do not compose. `expand_inner` flattens a composite tree into a single flat list of leaf command IDs and aggregates the scheduling flags across the whole traversal:

```rust
if c.parallel {
    ctx.any_parallel = true;
}
if !c.fail_fast {
    ctx.fail_fast_disabled = true;
}
```

Both are monotonic ORs over every composite visited, so:

- a `parallel = true` descendant silently promotes a `parallel = false` ancestor to parallel;
- a `fail_fast = false` descendant silently disables fail-fast for the whole plan.

The per-composite flags therefore describe intent that the runner cannot honour. There is no way to say "run these groups in order, but let the steps inside one group run together" — the most natural way to express a staged pipeline.

**How this was found**: while making the Rust `verify` sequential (PR #5), the obvious design was a sequential `verify` nesting a `parallel = true` `compile` group, keeping concurrency for the two compile steps while guaranteeing `fmt` finished first. The config looked correct and the built binary ran everything in parallel anyway — strictly worse than the status quo, because it *claimed* to be sequential. The workaround was to give up the nesting and flatten `verify`, accepting ~0.45s -> ~0.90s warm.

**Why it matters**: this is a silent correctness trap in the config language, not just a missing feature. `.ops.toml` authors get no error and no warning; the flag simply does not mean what it says, and the failure mode (racing steps) is intermittent. Every future stack default or user config that reaches for nesting hits it.

**Options to weigh** (decision needed, not prescribed):

1. **Preserve group boundaries** — schedule each composite as a unit so a child group's `parallel` applies only within it. Most expressive; largest change to the runner's plan model, which is currently a flat `Vec<CommandId>`.
2. **Make the ancestor win** (AND instead of OR) — a `parallel = false` anywhere forces the plan sequential. Small change, matches the principle of least surprise for a gate, but removes the ability to opt a subtree into concurrency.
3. **Reject at validation time** — keep the flat model and make a parallel descendant under a sequential ancestor a config error, so the trap is loud instead of silent. Cheapest, and could ship ahead of 1 or 2.

Option 3 is compatible with either of the others and would have caught this immediately.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A decision is recorded on which of the three options (or another) ops adopts, with the reasoning
- [x] #2 Per-composite parallel and fail_fast either behave as their names imply, or a config that nests them incompatibly is rejected with an actionable error
- [x] #3 The chosen behaviour is documented wherever composite commands are described
- [x] #4 Tests cover a nested composite whose child scheduling differs from its parent
- [x] #5 ops verify and ops qa pass
<!-- AC:END -->

## Decision

<!-- SECTION:DECISION:BEGIN -->
**Adopted: Option 3 — reject at validation time.** Decided 2026-08-15.

Keep the flat plan model. A composite tree that declares conflicting scheduling
flags is rejected during expansion with a new
`ExpandError::ConflictingSchedule`, instead of OR-folding the flags and running
something the config did not describe.

**Reasoning**

- The bug is *silence*, not the missing feature. A config that reads
  `parallel = false` and runs concurrently is worse than one that refuses to
  run: the failure mode is intermittent and easy to misattribute. Rejection
  removes the class immediately.
- Cheapest of the three by a wide margin — a validation pass in `expand_inner`
  plus an error variant. No change to the plan model, executor, event stream or
  progress rendering.
- **Forward-compatible with Option 1.** Rejecting today is a strict subset of
  what real group boundaries would later accept, so adopting Option 1 relaxes
  an error rather than changing behaviour under anyone's feet. Shipping Option 3
  first costs nothing if Option 1 is later funded.
- Option 2 (AND / ancestor wins) rejected: it keeps the flag lying, just in the
  safe direction. `lint.parallel = true` would have no effect inside `verify`
  *and* none standalone, which is its own surprise.
- Option 1 remains the right long-term answer — it is what recovers the
  concurrency both the Rust and Python `verify` plans gave up. Deferred on blast
  radius: the plan is a flat `Vec<CommandId>` consumed by `run_cmd.rs`,
  `events.rs`, the progress renderer, `--raw` output and dry-run, all of which
  assume a flat step list.

**Scope note.** Agreement is enforced in *both* directions, not just the
sequential-parent/parallel-child case in the report — under a flat plan a
sequential child under a parallel parent is equally unhonoured. Checked per
expansion root, so `ops run seq par` still merges two independently-valid plans
with differing flags.

**Follow-up.** Option 1 is not filed as a task; the constraint is documented in
`README.md` and on `CompositeCommandSpec`. File it if a real config needs staged
scheduling.
<!-- SECTION:DECISION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped in PR #14 (squashed as `81e514d`), released in v0.36.1.

`ExpandError::ConflictingSchedule` carries the flag name, both composite names
and both values. `ExpandCtx` gains `parallel_decl` / `fail_fast_decl` recording
the first composite to declare each flag; `check_schedule_flag` rejects any
later disagreement, called before recursing so the error names the shallowest
offender. Comparing against the first composite visited is sufficient for
whole-plan agreement — expansion is depth-first from the root, so the first
declaration is the root's, and if every later node matches the root then all
match each other — and it names the command the user actually invoked.

Rendered output:

```console
$ ops verify
ops: error: conflicting `parallel` in the plan for `verify`: `verify` sets parallel = false, but `lint` sets parallel = true
ops: error:   composite commands are flattened into one plan and scheduled as a single unit, so mixed `parallel` values cannot both be honoured
ops: error:   fix: make them agree — set `lint.parallel = false`, or set `verify.parallel = true`
```

**Tests.** New `schedule_flag_agreement_tests` module in
`crates/runner/src/command/tests/expand.rs` covers both conflict directions,
`fail_fast` disagreement, all four agreeing flag combinations, the aggregated
flags, a diamond revisit (an agreeing node visited twice must not read as a
conflict), and the error message content.

**Behaviour change to existing tests.** `merge_plan_picks_up_nested_parallel`
and `merge_plan_picks_up_nested_fail_fast_disabled` in
`crates/cli/src/run_cmd/tests.rs` asserted the old OR-fold; inverted to assert
rejection, with two cases added — nesting that *agrees* still expands, and
independent roots are still merged. The module doc records why they flipped.

**Docs (AC #3).** "Command groups and scheduling" section in `README.md` with
the rule, a failing example, the rendered error, the fix, the per-root scoping
note, and a note that staged scheduling is not expressible today. Field docs on
`CompositeCommandSpec::parallel` / `::fail_fast`.

**Semver note.** This is a breaking change for configs that nest groups with
differing flags. It shipped in v0.36.1, a *patch* release — the squash-merge
subject was a plain `fix:` and the `BREAKING CHANGE:` footer ended up mid-body
rather than trailing, so `cog bump --auto` did not see it. Flagged separately;
no semver signal reached users.
<!-- SECTION:NOTES:END -->
