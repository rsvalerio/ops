---
id: TASK-1657
title: >-
  Composite expansion ORs parallel, so nesting cannot express a sequential
  parent with a parallel child
status: Triage
assignee: []
created_date: '2026-08-06 14:30'
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
- [ ] #1 A decision is recorded on which of the three options (or another) ops adopts, with the reasoning
- [ ] #2 Per-composite parallel and fail_fast either behave as their names imply, or a config that nests them incompatibly is rejected with an actionable error
- [ ] #3 The chosen behaviour is documented wherever composite commands are described
- [ ] #4 Tests cover a nested composite whose child scheduling differs from its parent
- [ ] #5 ops verify and ops qa pass
<!-- AC:END -->
