---
id: TASK-1818
title: >-
  SEC-31: Config::validate_commands has no production caller, so the
  alias-collision rules never run in the shipped binary
status: To Do
assignee:
  - TASK-1983
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 14:08'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/config/root.rs
  - crates/core/src/config/loader/mod.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/root.rs:104` (`validate_commands`), `crates/core/src/config/loader/mod.rs:227` (the real load path)

**What**: A repo-wide grep for `validate_commands` returns hits in exactly two places: its own definition in `root.rs`, and `config/tests/validate_tests.rs`. Nothing in `crates/cli`, `crates/runner`, or any extension calls it.

The load path that every `ops` invocation traverses is `load_config_at`, which calls only `Config::validate()`. `validate()`'s own doc (root.rs:64-74) says it deliberately skips composites and defers to `validate_commands`. So the five checks `validate_commands` exists for — unknown composite reference, cycle, `MAX_COMPOSITE_DEPTH`, alias-collides-with-command-name, and duplicate-alias-across-commands — do not execute in the shipped CLI.

Three of the five are re-caught downstream: the runner's `expand_inner` (`crates/runner/src/command/resolve.rs:349`) rediscovers unknown refs, cycles, and depth at dispatch time. **The two alias-hygiene rules are duplicated nowhere.** `Config::resolve_alias` (root.rs:219) is an order-dependent linear scan that returns the first match, which is precisely the failure the comment at root.rs:121-129 promises is prevented:

> Catch both up-front so misconfigurations fail loud at validate time rather than as ghost behaviour at invocation.

The binary does not have that behaviour. A `.ops.toml` where two commands declare the same alias, or where an alias shadows a real command name, loads cleanly and then dispatches to whichever command happens to sit earlier in the `IndexMap` — silently running the wrong command.

**Why it matters**: SEC-31 fail-closed. A validation routine with eight dedicated tests, a documented contract, and zero callers reads to every future maintainer as live protection. The tests all pass, so nothing signals the gap. Meanwhile the specific misconfiguration it was written for (TASK-1181 / TASK-1182) reaches dispatch and silently selects the wrong command — the worst possible outcome for a tool that runs arbitrary shell commands on a developer's machine.

<!-- scan confidence: verified — `grep -rn 'validate_commands' --include='*.rs' .` over the whole repo returns only root.rs and validate_tests.rs -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_config_at (or a single documented setup path every CLI entry point traverses) invokes validate_commands with the stack-default plus extension-registered command ids as externals
- [ ] #2 A test asserts that a .ops.toml declaring the same alias on two commands fails through the real load entry point, not only through a direct validate_commands call
- [ ] #3 A test asserts that an alias shadowing an existing command name fails through the same real entry point
- [ ] #4 If the checks must stay opt-in because externals are unknown at load time, the doc comments at root.rs:88-103 and root.rs:121-129 are corrected to say so, and the two alias rules are relocated to a path that does run before dispatch
<!-- AC:END -->
