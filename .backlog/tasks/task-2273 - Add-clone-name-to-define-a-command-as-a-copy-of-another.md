---
id: TASK-2273
title: 'Add clone = "<name>" to define a command as a copy of another'
status: Triage
assignee: []
created_date: '2026-09-18 19:30'
labels:
  - feature
  - config
dependencies:
  - TASK-2272
modified_files:
  - crates/core/src/config/commands.rs
  - crates/core/src/config/extend.rs
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
There is no way to define a new command as a variant of an existing one (typically a stack default). dbsec has `fuzz/` as a separate workspace root and needs `fuzz-clippy`, `fuzz-build`, `fuzz-fmt`: the stack `clippy`/`build`/`fmt` plus `--manifest-path fuzz/Cargo.toml`. Today each is a hand-written exec spec that silently diverges when the stack default's flags change.

Proposal: `[commands.fuzz-clippy] clone = "clippy"` copies the resolved spec (config -> stack default -> extension) under the new name at load time. Scalar fields given alongside (`help`, `category`, `env`, `cwd`, `timeout_secs`, `exclusive`, `aliases`) override the copy. `args` and `program` are not accepted with `clone` -- extra args go through `[extend.fuzz-clippy] args = [...]` (TASK-2272), so the two features compose and there is one rule for where appended args land (before `--`).

Works for composites too (`clone = "verify"` then `[extend.<new>] commands = [...]`).

Resolution order matters: clone before extend, so `[extend.clippy]` in the same config either reaches the clone or not by a documented rule (suggest: the clone copies the source *before* the source's own extends, so extends stay per-name).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `clone = "<name>"` defines a command as a copy of a config, stack-default or extension command
- [ ] #2 Scalar fields beside `clone` override the copy; `program`/`args`/`commands` beside `clone` are load errors pointing at `[extend.<name>]`
- [ ] #3 `[extend.<clone>] args` / `commands` apply to the clone (depends on TASK-2272)
- [ ] #4 Unknown source, clone-of-clone cycles and cloning into an existing stack-default name are load errors naming both names
- [ ] #5 Whether the source's own `[extend.<source>]` reaches the clone is decided, documented and tested
- [ ] #6 `ops --dry-run <clone>` shows the resolved program and args
- [ ] #7 README documents clone with the fuzz-clippy example
<!-- AC:END -->
