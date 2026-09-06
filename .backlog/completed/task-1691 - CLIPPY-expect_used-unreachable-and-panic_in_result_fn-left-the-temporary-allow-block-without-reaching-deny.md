---
id: TASK-1691
title: >-
  CLIPPY: expect_used, unreachable and panic_in_result_fn left the
  temporary-allow block without reaching deny
status: Done
assignee: []
created_date: '2026-08-26 22:41'
updated_date: '2026-08-26 23:04'
labels:
  - code-review-rust
  - clippy
dependencies: []
modified_files:
  - Cargo.toml
  - docs/clippy.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `Cargo.toml` (the `[workspace.lints.clippy]` deny list)

**What**: TASK-1675 and TASK-1682 cleared their sites and deleted their lines from the
`# --- Temporary allows ---` block, which is what their acceptance criteria asked for.
But `expect_used`, `unreachable` and `panic_in_result_fn` are `restriction`-group lints,
and `[workspace.lints.clippy]` enables only `all`, `pedantic` and `nursery`. No group
turns a restriction lint on, so deleting its `allow` line returns it to clippy's default
level, which is `allow` — not `deny`. The gate stays green because the lint no longer
fires at all, and new `expect()` / `unreachable!()` / panicking `Result` fns land
unchallenged.

Both tasks' AC #2 also required "and the lint reaches the workspace at `deny`", so this is
the second half of an AC that the deletion alone did not satisfy.

The sibling restriction lints were handled correctly and show the shape of the fix:
`indexing_slicing` and `string_slice` (TASK-1672 / TASK-1673) moved up to the explicit
deny list next to `unwrap_used` and `panic`, and TASK-1671 / TASK-1674 did the same for
`arithmetic_side_effects` and `as_conversions`.

Every other lint that left the block belongs to `nursery` or `pedantic`, so plain deletion
was correct for those.

**Why it matters**: three deny-level bans on panicking code silently stopped being
enforced. `expect_used` in particular was worth 22 sites of cleanup across 14 files under
TASK-1675; nothing now stops them coming back. `clippy.toml` already carries
`allow-expect-in-tests = true`, so re-denying it should not re-flag test code.

**Fix**: add the three lines to the explicit deny list in `[workspace.lints.clippy]`,
then run `cargo clippy --workspace --all-features --all-targets -- -D warnings` and clear
whatever has crept back in since those waves landed. `docs/clippy.md` now documents the
restriction-lint distinction under "The temporary-allow block"; its layer-1 table needs
the three rows too.

**Origin**: discovered during TASK-1684 (wave142) while fixing TASK-1671 — the same trap
caught this wave first, and inspecting the landing branch showed three lints where it had
already gone unnoticed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 expect_used, unreachable and panic_in_result_fn each appear at deny in the [workspace.lints.clippy] deny list of the root Cargo.toml
- [ ] #2 cargo clippy --workspace --all-features --all-targets -- -D warnings passes with all three at deny
- [ ] #3 docs/clippy.md layer-1 table lists the three lints
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed on the run integration branch code-review/run-20260826 as commit 7a7e1f3, not deferred: all three lints are now in the [workspace.lints.clippy] deny list beside the other panic bans.

The single site this surfaced is the FailingIngestor test double in extensions/duckdb/src/sql/ingest/orchestrator.rs, whose load() must never run once collect() fails. allow-panic-in-tests does not cover panic_in_result_fn, so it carries a layer-3 #[allow] with the reason at the call site rather than being weakened to an Err return.

Verified: cargo clippy --workspace --all-targets --all-features clean; ops verify 7/7; 2405/2405 nextest; doctests clean.
<!-- SECTION:NOTES:END -->
