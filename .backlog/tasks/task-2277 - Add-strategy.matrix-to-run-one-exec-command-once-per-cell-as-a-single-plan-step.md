---
id: TASK-2277
title: 'Add strategy.matrix to run one exec command once per cell as a single plan step'
status: In Progress
assignee: []
created_date: '2026-09-26 12:27'
updated_date: '2026-09-26 13:42'
labels:
  - feature
  - config
dependencies: []
modified_files:
  - crates/core/src/config/commands.rs
  - crates/core/src/expand.rs
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
There is no way to run one exec command over a list of values. dbsec needs `cargo doc --no-deps -p <crate>` once per published crate (dbsec-core, dbsec, dbsec-pgwire, dbsec-vault, dbsec-derive), each invocation separate: cargo resolves features for all `-p` packages together, so a combined `cargo doc -p a -p b` documents dbsec-core with `keyfile` on (dbsec depends on it with that feature) and its default-feature build never happens. Today this lives in dbsec `scripts/doc-default-check.sh`, which `ops doc-default` wraps and dbsec `ops verify` appends via `[extend.verify]`.

Spelling it as five `.ops.toml` exec commands plus a group does not work:
- the group is nested in the parallel `verify`, so it must be `parallel = true` and share verify's `fail_fast = true`: the first failing crate stops the plan, so the other broken crates are only found one run at a time (the script deliberately runs all five and reports every failure);
- the five steps join the same parallel stage and block on one another's cargo target-dir lock (they share `${CARGO_TARGET_DIR:-target}/doc-default`), gaining nothing; marking them `exclusive` instead adds four stage barriers to verify;
- the crate list is spread over five near-identical blocks.

Proposal, modelled on GitHub Actions `strategy`:

```toml
[commands.doc-default]
program = "cargo"
args = ["doc", "--no-deps", "-p", "${matrix.crate}"]
env = { CARGO_TARGET_DIR = "${CARGO_TARGET_DIR:-target}/doc-default" }

[commands.doc-default.strategy]
matrix = { crate = ["dbsec-core", "dbsec", "dbsec-pgwire", "dbsec-vault", "dbsec-derive"] }
max_parallel = 1
fail_fast = false
```

Core rule: a matrix command is ONE step to the enclosing plan. Its `exclusive` applies to the whole matrix; it succeeds only when every cell succeeds. `strategy.fail_fast` / `max_parallel` govern only the cells, so they never conflict with the enclosing parallel plan's `fail_fast` (the same-plan agreement rule applies to groups, not to a matrix). This fits the per-group plan trees from TASK-2275: the matrix is a leaf with its own inner schedule.

Design points to settle:
- Several matrix keys form a Cartesian product; `include` / `exclude` add or drop cells (GHA semantics).
- `${matrix.<key>}` is substituted in args, env values and cwd in a dedicated pass before `Variables` expansion. It never falls back to the environment, and an unknown key (`${matrix.crat}`) or a `${matrix.*}` reference on a command with no strategy is a load error naming the command.
- `--dry-run` lists every cell with its fully expanded program/args. Progress renders one sub-row per cell labelled with its values (`doc-default [crate=dbsec-core]`); the failure summary names every failed cell.
- clone/extend: a clone copies the strategy; `[extend.<name>] matrix.<key> = [...]` appends values to a key; a `strategy` given beside `clone` replaces the copied one wholesale (lists/maps replace, as today).
- Exec commands only for v1; a matrix over a composite is out of scope.
- `--raw` runs cells sequentially, as it does everything else.
- Phase 2 (separate task): values from the workspace, e.g. `matrix.crate = { from = "workspace-crates", publish = true }` via cargo metadata (and Go modules / Node workspaces), so a new published crate is picked up instead of silently missed. Out of scope here but the schema should leave room for a table value.

First adopter: dbsec replaces `scripts/doc-default-check.sh` with the `[commands.doc-default]` above. (dbsec CI calls the script directly and does not install ops, so the script is only deleted there once CI runs `ops doc-default`; that is dbsec's call.)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `[commands.<name>.strategy] matrix = { key = [...] }` on an exec command runs it once per cell; several keys form the Cartesian product
- [x] #2 `include` / `exclude` add and drop cells with GitHub Actions semantics
- [x] #3 `${matrix.<key>}` is substituted in args, env values and cwd, never falls back to the environment, and an unknown key or a reference without a strategy is a load error naming the command
- [x] #4 A matrix command is one step to the enclosing plan: `exclusive` applies to the whole matrix, it fails when any cell fails, and its `strategy.fail_fast` does not trip the same-plan fail_fast agreement check when nested in a parallel group
- [x] #5 `max_parallel` bounds concurrent cells (1 = sequential); `strategy.fail_fast = false` runs every cell and the summary names every failed cell by its values
- [x] #6 `ops --dry-run <name>` lists every cell with its expanded program and args; progress shows one sub-row per cell
- [x] #7 clone copies the strategy, `[extend.<name>] matrix.<key>` appends values, and a strategy beside clone replaces the copy; each rule is tested
- [x] #8 README documents strategy.matrix with the dbsec doc-default example as the worked case
- [ ] #9 dbsec `ops doc-default` expressed as the matrix command above reproduces scripts/doc-default-check.sh: one cargo doc per crate, own target dir, all crates reported on failure, and runs inside dbsec `ops verify`

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented on branch feat/strategy-matrix (worktree ../ops-strategy-matrix). Core: config/strategy.rs (Strategy/Matrix, GHA cells, ${matrix.<key>} substitution, 256-cell cap, duplicate-value guard); ExecCommandSpec.strategy + matrix_cells + load-time validation; clone strategy override; [extend] matrix append (+ layer concat). Runner: command/matrix.rs driver (cells launched in order under their own semaphore/fail_fast, rows relabelled to cell ids, skipped/panicked synthesis, one aggregate StepResult); outer parallel fail_fast ignores cell StepFailed and trips on the aggregate result; row_ids() feeds PlanStarted; raw runs cells sequentially. CLI: display rows + dry-run cell listing. Axis order is by key name: CommandSpec deserializes through toml::Value (sorted tables), so declaration order is not recoverable without toml preserve_order. AC #9 verified on a scratch copy of dbsec with the matrix [commands.doc-default]: one cargo doc per crate in target/doc-default, in ops verify's plan; with broken links injected in dbsec-core (keyfile link) and dbsec-vault both were reported in one run, exit 1. Left unchecked until dbsec itself adopts the config.
<!-- SECTION:NOTES:END -->
