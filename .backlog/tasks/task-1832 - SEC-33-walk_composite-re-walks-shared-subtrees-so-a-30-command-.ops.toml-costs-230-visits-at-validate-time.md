---
id: TASK-1832
title: >-
  SEC-33: walk_composite re-walks shared subtrees, so a 30-command .ops.toml
  costs 2^30 visits at validate time
status: To Do
assignee:
  - TASK-1983
created_date: '2026-08-27 15:21'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/config/root.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/root.rs:164-210` (`Config::walk_composite`), entered from `crates/core/src/config/root.rs:104-119` (`Config::validate_commands`)

**What**: `walk_composite` is a depth-first cycle/reference checker that maintains `visiting` as a *path* set — it inserts `name` on entry and removes it on every exit path (the ERR-1 / TASK-1221 invariant documented at root.rs:154-163). That is correct for cycle detection, but it means the walker has **no memory of nodes it has already fully validated**. Every time a composite is reached along a different path, its entire subtree is walked again from scratch:

```rust
for sub in &c.commands {
    ...
    if let Some(CommandSpec::Composite(_)) = self.commands.get(sub_str) {
        let next = depth.saturating_add(1);
        if let Err(e) = self.walk_composite(sub_str, known, visiting, next) {
```

`CompositeCommandSpec::commands` is a plain `Vec<String>` with no de-duplication (commands.rs:374), and `MAX_COMPOSITE_DEPTH` is 100 (root.rs:15), so the following `.ops.toml` — 31 command entries, ~90 lines, well inside every size cap — makes validation run 2^30 recursive calls:

```toml
[commands.c0]
commands = ["c1", "c1"]
[commands.c1]
commands = ["c2", "c2"]
# … c2 … c29 …
[commands.c29]
commands = ["leaf"]
[commands.leaf]
program = "true"
```

Each level doubles the number of visits; depth 30 is nowhere near the depth-100 bail, so nothing stops it. The same blowup arises from any diamond-shaped composite graph, not just the literal duplicate above — the existing test `validate_commands_accepts_diamond_dag` (config/tests/validate_tests.rs:418) is exactly this shape at N=1 and passes precisely because re-walking `d` is treated as legal.

**Why it matters**: SEC-33. `.ops.toml` is repo-supplied content and `ops` is explicitly designed to run inside third-party repositories, so a config that hangs the validator is an unauthenticated local DoS: `ops <anything>` never returns, with no output and no timeout. Note the current blast radius is limited because `validate_commands` has no production caller today (see TASK-1818) — but TASK-1818's fix is to wire it into `load_config_at`, which puts this on the path of **every** `ops` invocation. Fixing the two together is the point.

The fix is the standard three-colour DFS: keep the existing path-set `visiting` for cycle detection and add a `validated: HashSet<&str>` of nodes whose subtree already returned `Ok`, skipping the recursion when the node is in it. That is sound — a node that completed with no cycle cannot acquire one by being reached from a second parent — and turns the walk from O(2^n) into O(V+E).

<!-- scan confidence: verified by reading root.rs:164-210; `visiting.remove(name)` at line 208 runs on every exit, and no other set records completed nodes -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 walk_composite records nodes whose subtree validated successfully and does not re-descend into them, so validation is O(V+E) rather than O(2^n)
- [ ] #2 Cycle detection is unchanged: validate_commands_rejects_self_cycle and validate_commands_rejects_indirect_cycle still pass, and validate_commands_accepts_diamond_dag still passes
- [ ] #3 A regression test builds a 30-level chain where each composite lists the next one twice and asserts validate_commands returns within a bounded wall-clock budget (or asserts a call/visit counter is linear in the command count)
- [ ] #4 The memoisation is documented alongside the existing ERR-1 / TASK-1221 visiting-set invariant so a future refactor does not silently drop it
<!-- AC:END -->
