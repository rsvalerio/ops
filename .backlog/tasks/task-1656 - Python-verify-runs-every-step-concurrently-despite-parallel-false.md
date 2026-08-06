---
id: TASK-1656
title: Python verify runs every step concurrently despite parallel = false
status: Triage
assignee: []
created_date: '2026-08-06 14:30'
labels:
  - bug
  - stack-defaults
  - python
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Found while** fixing the Rust stack defaults (PR #5), which hit the same mechanism.

**File**: `crates/core/src/.default.python.ops.toml`, `crates/runner/src/command/resolve.rs:279`

**What**: `commands.verify` sets `parallel = false`, but the plan it produces runs fully parallel. Composite expansion flattens the whole tree into one leaf plan and ORs the scheduling flags together (`ctx.any_parallel = true` in `expand_inner`), so the single `parallel = true` on `commands.lint` promotes the entire `verify` plan to parallel.

Python is the only stack with a nested composite today, and it is the one where the consequences bite:

- `fmt` is `["ruff-fix", "black-fmt"]` with `parallel = false` and help text "Run ruff --fix then black". That ordering is silently violated — `ruff check --fix .` and `black .` run **concurrently, both rewriting the same `.py` files**.
- `lint` runs `black --check .` concurrently with `fmt`'s `black .`, so a checker reads files while a formatter rewrites them.
- `type` (`pyright`) reads the same files while both formatters are mid-write.

The `fmt` composite's own sequencing guarantee cannot be honoured as long as any sibling in the tree is parallel.

**Why it matters**: two formatters mutating one file concurrently is a corruption/flake risk, not a tidiness issue — and the config reads as though it were safe. Anyone auditing `.default.python.ops.toml` sees `parallel = false` on `verify` and a documented "ruff --fix then black" ordering, neither of which holds at runtime.

**Quick fix** available independently of the semantics question: set `commands.lint.parallel = false` in the Python defaults. That restores sequential execution for the whole plan at the cost of running `ruff` and `black --check` serially. The deeper decision is tracked separately.

**Verify with**: the Rust stack's `rust_verify_is_sequential_so_fmt_cannot_race_the_compile_steps` test (added in PR #5) shows the shape of a regression guard that checks the transitive property rather than the flag.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Python 'verify' executes sequentially at runtime, not merely in config
- [ ] #2 'fmt' runs ruff-fix strictly before black-fmt, as its help text promises
- [ ] #3 No formatter runs concurrently with another formatter, a checker, or the type checker over the same files
- [ ] #4 A regression test asserts the transitive property (no parallel descendant under a sequential ancestor), not just the parent flag
- [ ] #5 ops verify and ops qa pass
<!-- AC:END -->
