---
id: TASK-2053
title: >-
  ARCH-11: ops verify lints only --all-features, so feature-off code can go red
  without failing the gate
status: Done
assignee:
  - TASK-2062
created_date: '2026-08-29 13:05'
updated_date: '2026-08-29 18:04'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - docs/clippy.md
  - .ops.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/clippy.md:350-353`

**What**: the workspace lint gate is
`cargo clippy --workspace --all-features --all-targets -- -D warnings`
(`ops verify`). With every feature on, `#[cfg(not(feature = ...))]` code is never
compiled, so lints that fire only in the feature-off build are invisible to the
gate. `docs/clippy.md` already warns that a plain `cargo clippy` misses
feature-gated code; the reverse gap — `--all-features` missing the *feature-off*
arms — is undocumented and unenforced.

TASK-2027 is the worked example: two `clippy::missing_const_for_fn` errors sat on
the `duckdb`-off `enrich_from_db` stubs in `ops-about` and only surfaced when
someone ran `cargo clippy -p ops-about`. A default-feature sweep over the
workspace is currently clean, so this is about keeping it that way rather than an
outstanding red build.

**Why it matters**: `-D clippy::nursery` is workspace policy, so any developer or
CI step that lints a subset of crates gets a red build from code they did not
touch, and the failure is attributed to whoever happened to run the narrower
command. Every crate with a `cfg(not(feature = ..))` arm can regress the same way.

**Origin**: discovered during TASK-2049 while fixing TASK-2027.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The lint gate (or a documented companion step) covers the default-feature build as well as --all-features, so a feature-off clippy error fails CI rather than only a per-crate run
- [x] #2 docs/clippy.md records the --all-features blind spot alongside the existing plain-cargo-clippy warning
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in code-review wave TASK-2062. AC #1: added [commands.clippy-default] to .ops.toml (cargo clippy --workspace --all-targets -- -D warnings) and wired it into run-before-push, and added a matching second step to the Lint job in .github/workflows/ci.yml so a feature-off clippy error now fails CI rather than only a per-crate run. It is deliberately NOT in ops verify: verify is the pre-commit gate and a second full-workspace clippy under a different feature fingerprint costs a second full compile on every commit; pre-push plus CI is the documented companion step the AC allows. AC #2: docs/clippy.md gained a "Two feature sets, two blind spots" section replacing "--all-features matters", with a table showing that plain cargo clippy and --all-features are two different builds and neither subsumes the other, the TASK-2027 duckdb-off missing_const_for_fn worked example, why the nursery policy makes the regression land on whoever next lints a subset of crates, and the ops clippy-default command; the intro also cross-links it from the gate command block. Verified the default-feature sweep is currently clean (cargo clippy --workspace --all-targets -- -D warnings, no findings).
<!-- SECTION:NOTES:END -->
