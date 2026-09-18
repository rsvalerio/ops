---
id: TASK-2272
title: 'Let [extend.<name>] append args to an exec command'
status: Triage
assignee: []
created_date: '2026-09-18 19:30'
labels:
  - feature
  - config
dependencies: []
modified_files:
  - crates/core/src/config/extend.rs
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today `[extend.<name>]` only takes `commands` and rejects exec targets (`crates/core/src/config/extend.rs`, "target is an exec command; only composites ... can be extended"). A workspace that needs one extra flag on a stack-default exec command has to redefine it wholesale: `[commands.clippy]` with no `program` fails with "missing field `program`", so the full `program`/`args` must be copied and then goes stale when the stack default changes -- the same problem `[extend.verify]` solved for composites.

Motivating case (dbsec): CI runs `cargo doc --no-deps --document-private-items --all-features` and dbsec pinned its own `[commands.doc]` copy so the local gate cannot drift from CI; clippy/build want `--locked` in CI-parity gates.

Proposal: `[extend.<exec-name>] args = [...]` appends to the target's `args`, with the same layering rules as `commands` (concatenated across global -> .ops.toml -> .ops.d; a local `[commands.<name>]` wins and is then extended; stack default is cloned, appended, inserted).

Design point to settle: the rust-stack `clippy` is `clippy --workspace --all-features --all-targets -- -D warnings`, so a naive append lands after `--` and becomes a lint flag, not a cargo flag. Either insert before the first `--` when one is present, or offer an explicit position (e.g. `args_before_separator` / `prepend_args`). Silent misplacement is the failure to avoid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `[extend.<exec>] args = [...]` appends to a stack-default or config-defined exec command's args
- [ ] #2 Appended args land before a `--` separator in the target (or the chosen positional rule is explicit and documented); a test pins the clippy `-- -D warnings` case
- [ ] #3 `commands` on an exec target and `args` on a composite target are load errors naming the target
- [ ] #4 Extends concatenate across config layers, as `commands` already do
- [ ] #5 `ops --dry-run <name>` shows the merged args
- [ ] #6 README "Extending existing commands" documents the args form
<!-- AC:END -->
