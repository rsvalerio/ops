---
id: TASK-1682
title: 'CLIPPY: clear the small-count lint tail and drop 14 workspace allows'
status: Done
assignee:
  - TASK-1685
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 21:52'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 50 pre-existing sites for this lint family across 30 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::needless_collect`, `clippy::collection_is_never_read`, `clippy::equatable_if_let`, `clippy::literal_string_with_formatting_args`, `clippy::needless_pass_by_ref_mut`, `clippy::unreachable`, `clippy::future_not_send`, `clippy::redundant_clone`, `clippy::panic_in_result_fn`, `clippy::single_option_map`, `clippy::derive_partial_eq_without_eq`, `clippy::branches_sharing_code`, `clippy::or_fun_call`, `clippy::iter_on_single_items`
**Sites**: 50 across 30 files

The fourteen lints that fired on six sites or fewer, grouped because each is too small for its own task. Fixing them removes fourteen lines from the temporary-allow block in one pass.

One caveat: `future_not_send` (2 sites, `crates/runner/src/command/`) is the only hard one — the futures capture a non-`Send` type and making them `Send` may not be worth it. If it is not, that lint alone keeps its allow with a reason and the other thirteen still go to deny.

## Scope

| File | Sites |
|---|---|
| `crates/runner/src/command/tests/exec.rs` | 5 |
| `extensions-rust/cargo-toml/src/inheritance.rs` | 5 |
| `crates/runner/src/command/tests/parallel_infra.rs` | 3 |
| `extensions/duckdb/src/sql/ingest/orchestrator.rs` | 3 |
| `crates/core/src/config/theme_types.rs` | 2 |
| `crates/runner/src/command/tests/build_cmd.rs` | 2 |
| `crates/runner/src/command/tests/events.rs` | 2 |
| `crates/runner/src/command/tests/parallel.rs` | 2 |
| `crates/runner/src/command/tests/sequential.rs` | 2 |
| `crates/runner/src/display.rs` | 2 |
| `crates/runner/src/display/finalize.rs` | 2 |
| `extensions-java/about/src/maven/pom.rs` | 2 |
| `crates/cli/src/args.rs` | 1 |
| `crates/cli/src/help.rs` | 1 |
| `crates/cli/src/test_utils.rs` | 1 |
| `crates/cli/tests/integration.rs` | 1 |
| `crates/core/src/config/commands.rs` | 1 |
| `crates/core/src/project_identity/card.rs` | 1 |
| `crates/core/src/subprocess/drain.rs` | 1 |
| `crates/runner/src/command/mod.rs` | 1 |
| `crates/runner/src/command/parallel.rs` | 1 |
| `crates/runner/src/display/tests.rs` | 1 |
| `crates/theme/src/tests/deserialize.rs` | 1 |
| `extensions-node/about/src/lib.rs` | 1 |
| `extensions-node/about/src/units.rs` | 1 |
| `extensions-python/about/src/lib.rs` | 1 |
| `extensions-rust/about/src/query.rs` | 1 |
| `extensions-rust/cargo-toml/src/types.rs` | 1 |
| `extensions-rust/metadata/src/tests/edge_cases.rs` | 1 |
| `extensions/about/src/manifest_cache.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `needless_collect`, `collection_is_never_read`, `equatable_if_let`, `literal_string_with_formatting_args`, `needless_pass_by_ref_mut`, `unreachable`, `future_not_send`, `redundant_clone`, `panic_in_result_fn`, `single_option_map`, `derive_partial_eq_without_eq`, `branches_sharing_code`, `or_fun_call`, `iter_on_single_items` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-1685 (wave143): all 14 lines deleted from the temporary-allow block, including `future_not_send`. Notable calls: `future_not_send` was not made `Send` — the `on_event: &mut impl FnMut(RunnerEvent)` sink is backed by non-`Send` indicatif state in the CLI, so a `+ Send` bound would rule out the display the binary actually uses; the two futures carry `#[allow(clippy::future_not_send)]` at layer 3 with that reason instead of keeping the workspace allow. `unreachable!` removed at all 3 sites by making the fallback total (card.rs renders an empty field, pom.rs returns `false`, the duckdb test mock uses `panic!`, which `allow-panic-in-tests` covers). `needless_collect` kept with a documented `#[allow]` at 3 sites where the collect is load-bearing (ends a borrow before a reassignment; spawns all threads before joining). `panic_in_result_fn` allowed at the two duckdb test mocks where the panic is the behaviour under test. Everything else fixed outright: `equatable_if_let` -> `matches!`/`==`, `single_option_map` -> caller-side `.map()`, `needless_pass_by_ref_mut` -> `&self`, `collection_is_never_read` -> no-op event sinks, plus `redundant_clone`, `or_fun_call`, `iter_on_single_items`, `branches_sharing_code`, `derive_partial_eq_without_eq`. `literal_string_with_formatting_args` allowed with a reason at the 4 indicatif-template sites.
<!-- SECTION:NOTES:END -->
